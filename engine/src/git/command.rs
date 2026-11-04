//! Safe, bounded execution of the installed Git executable.
//!
//! This module is deliberately the only place in the engine's Git boundary
//! that starts a process. Callers provide fixed argument vectors; no shell is
//! invoked and caller-controlled values are never concatenated into a command
//! string. Output is drained asynchronously while only bounded output is
//! retained in memory: stdout keeps a prefix and stderr keeps a tail for error
//! classification without retaining unbounded progress logs.

use std::{
    ffi::OsString,
    io,
    path::Path,
    process::{ExitStatus, Stdio},
    time::Duration,
};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
    task::JoinHandle,
    time::timeout,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const DEFAULT_STDOUT_LIMIT: usize = 4 * 1024 * 1024;
const DEFAULT_STDERR_LIMIT: usize = 64 * 1024;
const MAX_STDOUT_LIMIT: usize = 16 * 1024 * 1024;
const MAX_STDERR_LIMIT: usize = 1024 * 1024;
const MAX_COMMAND_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const STREAM_DRAIN_GRACE: Duration = Duration::from_secs(2);
const READ_BUFFER_SIZE: usize = 8 * 1024;

/// Operational limits for every Git subprocess.
///
/// Limits apply to retained output, not to how much output the child may write:
/// readers continue draining the pipes after the prefix limit is reached, so a
/// large command cannot deadlock because the child filled a pipe.
#[derive(Debug, Clone)]
pub struct GitCommandConfig {
    pub timeout: Duration,
    pub stdout_limit_bytes: usize,
    pub stderr_limit_bytes: usize,
}

impl Default for GitCommandConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            stdout_limit_bytes: DEFAULT_STDOUT_LIMIT,
            stderr_limit_bytes: DEFAULT_STDERR_LIMIT,
        }
    }
}

/// Errors from starting, waiting for, or collecting a Git process.
#[derive(Debug, Error)]
pub enum GitCommandError {
    #[error("Git executable was not found while attempting `{operation}`")]
    ExecutableNotFound { operation: &'static str },

    #[error("could not start Git while attempting `{operation}`: {source}")]
    Spawn {
        operation: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("Git operation `{operation}` timed out after {timeout_ms} ms")]
    Timeout {
        operation: &'static str,
        timeout_ms: u128,
    },

    #[error("could not wait for Git operation `{operation}`: {source}")]
    Wait {
        operation: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("Git operation `{operation}` exited with status {status:?}")]
    Failed {
        operation: &'static str,
        status: Option<i32>,
    },

    #[error("Git output collection failed for `{operation}` ({stream}): {source}")]
    StreamRead {
        operation: &'static str,
        stream: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("Git output reader task failed for `{operation}` ({stream})")]
    StreamTask {
        operation: &'static str,
        stream: &'static str,
    },

    #[error("Git output reader did not close after `{operation}` ({stream})")]
    StreamDrainTimeout {
        operation: &'static str,
        stream: &'static str,
    },

    #[error("invalid Git command limits for `{operation}`")]
    InvalidConfig { operation: &'static str },
}

/// Captured output. The byte vectors are bounded by `GitCommandConfig`.
#[derive(Debug)]
pub(super) struct GitCommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: ExitStatus,
    pub stdout_truncated: bool,
}

impl GitCommandOutput {
    pub fn ensure_success(&self, operation: &'static str) -> Result<(), GitCommandError> {
        if self.status.success() {
            Ok(())
        } else {
            // Never surface raw stderr: Git diagnostics can echo a remote URL
            // or a credential-helper detail. The exit code remains actionable.
            Err(GitCommandError::Failed {
                operation,
                status: self.status.code(),
            })
        }
    }
}

struct CapturedBytes {
    bytes: Vec<u8>,
    truncated: bool,
}

/// Run Git with fixed arguments, non-interactive settings, and bounded capture.
pub(super) async fn run_git(
    operation: &'static str,
    args: Vec<OsString>,
    cwd: Option<&Path>,
    config: &GitCommandConfig,
) -> Result<GitCommandOutput, GitCommandError> {
    if config.timeout.is_zero()
        || config.timeout > MAX_COMMAND_TIMEOUT
        || config.stdout_limit_bytes == 0
        || config.stdout_limit_bytes > MAX_STDOUT_LIMIT
        || config.stderr_limit_bytes == 0
        || config.stderr_limit_bytes > MAX_STDERR_LIMIT
    {
        return Err(GitCommandError::InvalidConfig { operation });
    }

    let mut command = Command::new("git");
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "")
        .env("SSH_ASKPASS", "")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_git_config_path())
        .env("GIT_CONFIG_COUNT", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env_remove("GIT_SSH_COMMAND")
        .env_remove("GIT_SSH")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_NAMESPACE")
        .env_remove("GIT_TEMPLATE_DIR")
        .env_remove("GIT_EXEC_PATH")
        .env_remove("GIT_CONFIG_SYSTEM")
        .env_remove("GIT_TRACE")
        .env_remove("GIT_TRACE_SETUP")
        .env_remove("GIT_TRACE_PACKET")
        .env_remove("GIT_TRACE_PERFORMANCE")
        .env_remove("GIT_TRACE_CURL")
        .env_remove("GIT_TRACE_CURL_NO_DATA")
        .env_remove("GIT_TRACE2")
        .env_remove("GIT_TRACE2_EVENT")
        .env_remove("GIT_TRACE2_PERF")
        .env_remove("GIT_TRACE2_BRIEF")
        .env_remove("GIT_PROXY_COMMAND")
        .env_remove("GIT_PREFIX")
        .env_remove("GIT_REPLACE_REF_BASE")
        .env_remove("GIT_CURL_VERBOSE")
        .env_remove("GIT_SSL_NO_VERIFY");
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let mut child = command.spawn().map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            GitCommandError::ExecutableNotFound { operation }
        } else {
            GitCommandError::Spawn { operation, source }
        }
    })?;

    let stdout = child.stdout.take().ok_or(GitCommandError::StreamTask {
        operation,
        stream: "stdout",
    })?;
    let stderr = child.stderr.take().ok_or(GitCommandError::StreamTask {
        operation,
        stream: "stderr",
    })?;

    let stdout_task = tokio::spawn(read_capped(stdout, config.stdout_limit_bytes));
    // Keep the tail of stderr: fatal authentication/network diagnostics usually
    // occur at the end, while progress output may precede them.
    let stderr_task = tokio::spawn(read_tail(stderr, config.stderr_limit_bytes));

    let status = match timeout(config.timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(source)) => {
            terminate_child(&mut child).await;
            stdout_task.abort();
            stderr_task.abort();
            return Err(GitCommandError::Wait { operation, source });
        }
        Err(_) => {
            terminate_child(&mut child).await;
            stdout_task.abort();
            stderr_task.abort();
            return Err(GitCommandError::Timeout {
                operation,
                timeout_ms: config.timeout.as_millis(),
            });
        }
    };

    let stdout_result = await_capture(stdout_task, operation, "stdout").await;
    let stderr_result = await_capture(stderr_task, operation, "stderr").await;
    let stdout = stdout_result?;
    let stderr = stderr_result?;

    Ok(GitCommandOutput {
        stdout: stdout.bytes,
        stderr: stderr.bytes,
        status,
        stdout_truncated: stdout.truncated,
    })
}

pub(super) async fn run_git_checked(
    operation: &'static str,
    args: Vec<OsString>,
    cwd: Option<&Path>,
    config: &GitCommandConfig,
) -> Result<GitCommandOutput, GitCommandError> {
    let output = run_git(operation, args, cwd, config).await?;
    output.ensure_success(operation)?;
    Ok(output)
}

pub(super) fn output_text<'a>(
    output: &'a GitCommandOutput,
    operation: &'static str,
) -> Result<&'a str, super::GitError> {
    if output.stdout_truncated {
        return Err(super::GitError::OutputLimitExceeded {
            operation,
            stream: "stdout",
        });
    }
    std::str::from_utf8(&output.stdout).map_err(|_| super::GitError::MalformedOutput { operation })
}

pub(super) fn output_line<'a>(
    output: &'a GitCommandOutput,
    operation: &'static str,
) -> Result<&'a str, super::GitError> {
    Ok(output_text(output, operation)?.trim_end_matches(['\r', '\n']))
}

async fn read_capped<R>(mut reader: R, limit: usize) -> io::Result<CapturedBytes>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut retained = Vec::with_capacity(limit.min(READ_BUFFER_SIZE));
    let mut buffer = [0_u8; READ_BUFFER_SIZE];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(retained.len());
        let keep = remaining.min(read);
        retained.extend_from_slice(&buffer[..keep]);
        if keep < read {
            truncated = true;
        }
    }
    Ok(CapturedBytes {
        bytes: retained,
        truncated,
    })
}

async fn read_tail<R>(mut reader: R, limit: usize) -> io::Result<CapturedBytes>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    use std::collections::VecDeque;

    let mut retained = VecDeque::with_capacity(limit.min(READ_BUFFER_SIZE));
    let mut buffer = [0_u8; READ_BUFFER_SIZE];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        for byte in &buffer[..read] {
            if retained.len() == limit {
                retained.pop_front();
                truncated = true;
            }
            retained.push_back(*byte);
        }
    }
    Ok(CapturedBytes {
        bytes: retained.into_iter().collect(),
        truncated,
    })
}

async fn await_capture(
    task: JoinHandle<io::Result<CapturedBytes>>,
    operation: &'static str,
    stream: &'static str,
) -> Result<CapturedBytes, GitCommandError> {
    let mut task = task;
    match timeout(STREAM_DRAIN_GRACE, &mut task).await {
        Err(_) => {
            task.abort();
            Err(GitCommandError::StreamDrainTimeout { operation, stream })
        }
        Ok(Err(_)) => Err(GitCommandError::StreamTask { operation, stream }),
        Ok(Ok(Err(source))) => Err(GitCommandError::StreamRead {
            operation,
            stream,
            source,
        }),
        Ok(Ok(Ok(captured))) => Ok(captured),
    }
}

async fn terminate_child(child: &mut Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

#[cfg(unix)]
fn null_git_config_path() -> &'static str {
    "/dev/null"
}

#[cfg(windows)]
fn null_git_config_path() -> &'static str {
    "NUL"
}

#[cfg(not(any(unix, windows)))]
fn null_git_config_path() -> &'static str {
    ""
}

#[cfg(test)]
mod tests {
    use super::read_capped;
    use tokio::io::{AsyncWriteExt, duplex};

    #[tokio::test]
    async fn retained_pipe_output_is_bounded_while_the_stream_is_drained() {
        let (mut writer, reader) = duplex(32);
        let writer_task = tokio::spawn(async move {
            writer
                .write_all(b"0123456789abcdef")
                .await
                .expect("write fixture output");
        });
        let captured = read_capped(reader, 5).await.expect("read bounded output");
        writer_task.await.expect("writer task");
        assert_eq!(captured.bytes, b"01234");
        assert!(captured.truncated);
    }
}
