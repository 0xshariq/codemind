//! Commit-to-commit path diffing with explicit bounds and provenance.

use super::repository::GitRepository;
use super::{GitCommandConfig, GitError, command::run_git, validate_commit_id};
use crate::repository::RepositoryIdentity;
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, path::Path};

const MAX_CHANGED_PATHS_LIMIT: usize = 50_000;

/// A repository-relative path from Git. `display` is valid UTF-8 with lossy
/// replacement for display when necessary; `raw_bytes_hex` preserves the exact
/// path bytes when the original path was not valid UTF-8.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GitPath {
    pub display: String,
    pub raw_bytes_hex: Option<String>,
}

impl GitPath {
    fn from_git_bytes(bytes: &[u8]) -> Self {
        match std::str::from_utf8(bytes) {
            Ok(path) => Self {
                display: path.to_owned(),
                raw_bytes_hex: None,
            },
            Err(_) => Self {
                display: String::from_utf8_lossy(bytes).into_owned(),
                raw_bytes_hex: Some(hex_bytes(bytes)),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Other(String),
}

/// One repository-relative path change. For renames/copies, both old and new
/// paths are provided. For ordinary changes, only `new_path` or `old_path` is
/// present as appropriate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GitPathChange {
    pub kind: GitChangeKind,
    pub status_code: String,
    pub old_path: Option<GitPath>,
    pub new_path: Option<GitPath>,
}

#[derive(Debug, Clone, Copy)]
pub struct GitDiffOptions {
    /// Maximum changes returned. Truncation is reflected by `GitDiff::complete`.
    pub max_changed_paths: usize,
}

impl Default for GitDiffOptions {
    fn default() -> Self {
        Self {
            max_changed_paths: 20_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GitDiff {
    /// Stable repository identity for the prepared Git checkout. This is not
    /// derived from the temporary workspace path.
    pub repository_id: String,
    pub base_commit: String,
    pub target_commit: String,
    pub changes: Vec<GitPathChange>,
    pub complete: bool,
    /// Lower bound on omitted path changes. If the command output was capped,
    /// the exact number omitted is unknown.
    pub omitted_change_count: u64,
}

/// Compare two commit trees available in a prepared checkout. If the base is
/// absent (e.g. the remote was force-pushed or history is shallow), the caller
/// receives `BaseCommitUnavailable` and must choose a full re-analysis.
pub async fn diff_commits(
    repository: &GitRepository,
    base_commit: &str,
    target_commit: &str,
    options: GitDiffOptions,
    config: &GitCommandConfig,
) -> Result<GitDiff, GitError> {
    if options.max_changed_paths == 0 || options.max_changed_paths > MAX_CHANGED_PATHS_LIMIT {
        return Err(GitError::InvalidLimits);
    }
    let repository_id = repository_identity(repository)?;
    let base_commit = validate_commit_id(base_commit)?;
    let target_commit = validate_commit_id(target_commit)?;
    ensure_commit_available(repository.root(), &base_commit, true, config).await?;
    ensure_commit_available(repository.root(), &target_commit, false, config).await?;
    ensure_base_is_ancestor(repository.root(), &base_commit, &target_commit, config).await?;

    let args = vec![
        OsString::from("diff"),
        OsString::from("--no-ext-diff"),
        OsString::from("--no-textconv"),
        OsString::from("--name-status"),
        OsString::from("-z"),
        OsString::from("--find-renames"),
        OsString::from(base_commit.as_str()),
        OsString::from(target_commit.as_str()),
        OsString::from("--"),
    ];
    let output = run_git("commit-diff", args, Some(repository.root()), config).await?;
    output.ensure_success("commit-diff")?;
    let parsed = parse_name_status_z(&output.stdout, options.max_changed_paths);
    let mut omitted_change_count = parsed.omitted_change_count;
    if (output.stdout_truncated || !parsed.complete) && omitted_change_count == 0 {
        omitted_change_count = 1;
    }

    Ok(GitDiff {
        repository_id,
        base_commit,
        target_commit,
        changes: parsed.changes,
        complete: parsed.complete && !output.stdout_truncated,
        omitted_change_count,
    })
}

fn repository_identity(repository: &GitRepository) -> Result<String, GitError> {
    if let Some(remote_locator) = repository.remote_locator() {
        return Ok(RepositoryIdentity::from_remote(remote_locator)?
            .repository_id()
            .to_owned());
    }
    Ok(RepositoryIdentity::from_local_directory(repository.root())?
        .repository_id()
        .to_owned())
}

async fn ensure_commit_available(
    root: &Path,
    commit: &str,
    is_base: bool,
    config: &GitCommandConfig,
) -> Result<(), GitError> {
    let expression = format!("{commit}^{{commit}}");
    let args = vec![
        OsString::from("cat-file"),
        OsString::from("-e"),
        OsString::from(expression),
    ];
    let output = run_git("commit-availability", args, Some(root), config).await?;
    if output.status.success() {
        Ok(())
    } else if is_base {
        Err(GitError::BaseCommitUnavailable {
            commit: commit.to_owned(),
        })
    } else {
        Err(GitError::TargetCommitUnavailable {
            commit: commit.to_owned(),
        })
    }
}

/// Require a linear ancestry relationship before returning an incremental diff.
/// Two commits can both exist locally while belonging to diverging histories;
/// comparing their trees is valid Git behavior but is not a safe incremental
/// update base for CodeMind.
async fn ensure_base_is_ancestor(
    root: &Path,
    base_commit: &str,
    target_commit: &str,
    config: &GitCommandConfig,
) -> Result<(), GitError> {
    let args = vec![
        OsString::from("merge-base"),
        OsString::from("--is-ancestor"),
        OsString::from(base_commit),
        OsString::from(target_commit),
    ];
    let output = run_git("commit-ancestry", args, Some(root), config).await?;
    if output.status.success() {
        return Ok(());
    }
    if output.status.code() == Some(1) {
        return Err(GitError::BaseCommitNotAncestor {
            base_commit: base_commit.to_owned(),
            target_commit: target_commit.to_owned(),
        });
    }
    output.ensure_success("commit-ancestry")?;
    Ok(())
}

/// Parsed path changes. Only `max_changes` records are retained; the rest are
/// counted while the NUL-delimited output is streamed, so a large result cannot
/// expand into an unbounded `Vec` of path objects.
pub(super) struct ParsedPathChanges {
    pub changes: Vec<GitPathChange>,
    /// False if a record is malformed or any valid record was omitted by bounds.
    pub complete: bool,
    /// Count of parsed-but-unretained records, or at least one for malformed data.
    pub omitted_change_count: u64,
}

/// Parse `git diff-tree/diff --name-status -z` output. Git's `-z` form keeps
/// path bytes NUL-delimited and avoids quoting/escaping ambiguities. This parser
/// retains only the requested number of records and does not allocate a token
/// vector proportional to the whole output.
pub(super) fn parse_name_status_z(bytes: &[u8], max_changes: usize) -> ParsedPathChanges {
    let mut tokens = bytes.split(|byte| *byte == 0);
    let mut changes = Vec::with_capacity(max_changes.min(8_192));
    let mut complete = true;
    let mut omitted_change_count = 0_u64;

    while let Some(status_bytes) = tokens.next() {
        if status_bytes.is_empty() {
            // `-z` output commonly ends with a NUL terminator.
            continue;
        }

        let status_first = status_bytes.first().copied().unwrap_or_default();
        let is_rename_or_copy = matches!(status_first, b'R' | b'C');
        let is_deleted = status_first == b'D';
        let paths = if is_rename_or_copy {
            match (tokens.next(), tokens.next()) {
                (Some(old_path), Some(new_path))
                    if !old_path.is_empty() && !new_path.is_empty() =>
                {
                    Some((Some(old_path), Some(new_path)))
                }
                _ => None,
            }
        } else {
            tokens.next().and_then(|path| {
                if path.is_empty() {
                    None
                } else if is_deleted {
                    Some((Some(path), None))
                } else {
                    Some((None, Some(path)))
                }
            })
        };

        let Some((old_path, new_path)) = paths else {
            complete = false;
            omitted_change_count = omitted_change_count.saturating_add(1);
            break;
        };

        if changes.len() < max_changes {
            let status_code = String::from_utf8_lossy(status_bytes).into_owned();
            let kind = match status_first {
                b'A' => GitChangeKind::Added,
                b'M' => GitChangeKind::Modified,
                b'D' => GitChangeKind::Deleted,
                b'R' => GitChangeKind::Renamed,
                b'C' => GitChangeKind::Copied,
                b'T' => GitChangeKind::TypeChanged,
                b'U' => GitChangeKind::Unmerged,
                _ => GitChangeKind::Other(status_code.clone()),
            };
            changes.push(GitPathChange {
                kind,
                status_code,
                old_path: old_path.map(GitPath::from_git_bytes),
                new_path: new_path.map(GitPath::from_git_bytes),
            });
        } else {
            complete = false;
            omitted_change_count = omitted_change_count.saturating_add(1);
        }
    }

    ParsedPathChanges {
        changes,
        complete,
        omitted_change_count,
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{GitChangeKind, parse_name_status_z};

    #[test]
    fn parses_add_modify_delete_and_rename_records() {
        let output = concat!(
            "A\0src/new.rs\0M\0src/lib.rs\0D\0src/old.rs\0",
            "R100\0src/old_name.rs\0src/new_name.rs\0",
        )
        .as_bytes();
        let parsed = parse_name_status_z(output, 10);
        assert!(parsed.complete);
        assert_eq!(parsed.changes.len(), 4);
        assert_eq!(parsed.changes[0].kind, GitChangeKind::Added);
        assert_eq!(parsed.changes[1].kind, GitChangeKind::Modified);
        assert_eq!(parsed.changes[2].kind, GitChangeKind::Deleted);
        assert_eq!(parsed.changes[3].kind, GitChangeKind::Renamed);
        assert_eq!(
            parsed.changes[3]
                .old_path
                .as_ref()
                .map(|path| path.display.as_str()),
            Some("src/old_name.rs")
        );
        assert_eq!(
            parsed.changes[3]
                .new_path
                .as_ref()
                .map(|path| path.display.as_str()),
            Some("src/new_name.rs")
        );
    }

    #[test]
    fn malformed_or_truncated_rename_is_marked_incomplete() {
        let parsed = parse_name_status_z(b"R100\0old.rs\0", 10);
        assert!(parsed.changes.is_empty());
        assert!(!parsed.complete);
        assert_eq!(parsed.omitted_change_count, 1);
    }

    #[test]
    fn non_utf8_paths_preserve_original_bytes() {
        let parsed = parse_name_status_z(b"A\0src/\xff.rs\0", 10);
        assert!(parsed.complete);
        assert_eq!(parsed.changes.len(), 1);
        assert_eq!(
            parsed.changes[0]
                .new_path
                .as_ref()
                .and_then(|path| path.raw_bytes_hex.as_deref()),
            Some("7372632fff2e7273")
        );
    }

    #[test]
    fn parser_retains_only_the_configured_number_of_changes() {
        let parsed = parse_name_status_z(b"A\0a.rs\0M\0b.rs\0D\0c.rs\0", 1);
        assert!(!parsed.complete);
        assert_eq!(parsed.changes.len(), 1);
        assert_eq!(parsed.omitted_change_count, 2);
    }
}
