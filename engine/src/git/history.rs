//! Bounded extraction of Git commit ancestry and per-commit path changes.
//!
//! These records describe actual Git objects. They are intentionally distinct
//! from `source::snapshot` provenance and are not themselves graph nodes; a
//! later History Graph builder will transform them into canonical graph facts.

use super::{
    GitCommandConfig, GitError, GitPathChange,
    command::{output_line, output_text, run_git_checked},
    diff::{ParsedPathChanges, parse_name_status_z},
    repository::GitRepository,
    validate_commit_id,
};
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, path::Path};

const MAX_HISTORY_COMMITS: usize = 1_000;
const MAX_PATHS_PER_COMMIT: usize = 50_000;
const MAX_HISTORY_TOTAL_PATHS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitHistoryOptions {
    /// Maximum commit records to return. One additional commit is queried to
    /// determine whether the response is truncated at this bound.
    pub max_commits: usize,
    /// Maximum changed paths retained for each commit.
    pub max_changed_paths_per_commit: usize,
    /// Maximum changed paths retained across the complete history scan.
    pub max_total_paths: usize,
}

impl Default for GitHistoryOptions {
    fn default() -> Self {
        Self {
            max_commits: 100,
            max_changed_paths_per_commit: 5_000,
            max_total_paths: MAX_HISTORY_TOTAL_PATHS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GitCommit {
    pub commit_id: String,
    pub parent_commit_ids: Vec<String>,
    pub committed_at_unix_seconds: i64,
    pub author: String,
    pub subject: String,
    pub changed_paths: Vec<GitPathChange>,
    pub changed_paths_complete: bool,
    /// Lower bound on the number of change records omitted by the per-commit
    /// or overall history path limit. If Git output itself was
    /// truncated, more may have been omitted.
    pub omitted_paths_at_least: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GitHistory {
    pub head_commit: String,
    /// Commits are ordered newest-first in Git's topological traversal.
    pub commits: Vec<GitCommit>,
    /// False if ancestry is shallow/bounded or any per-commit output/path limit
    /// or the overall retained-path limit was hit.
    pub complete: bool,
    /// Lower bound on missing commit records when the configured limit is hit
    /// or a shallow repository hides older ancestry. The exact total is not
    /// scanned after a bound is reached.
    pub omitted_commits_at_least: u64,
}

/// Read recent commit ancestry and each commit's changed paths from the
/// temporary clone's object database. A bounded scan is labelled incomplete
/// rather than being presented as the repository's complete history.
pub async fn read_history(
    repository: &GitRepository,
    options: GitHistoryOptions,
    config: &GitCommandConfig,
) -> Result<GitHistory, GitError> {
    if options.max_commits == 0
        || options.max_commits > MAX_HISTORY_COMMITS
        || options.max_changed_paths_per_commit == 0
        || options.max_changed_paths_per_commit > MAX_PATHS_PER_COMMIT
        || options.max_total_paths == 0
        || options.max_total_paths > MAX_HISTORY_TOTAL_PATHS
    {
        return Err(GitError::InvalidLimits);
    }

    let is_shallow = repository_is_shallow(repository.root(), config).await?;
    let requested_count = options.max_commits.saturating_add(1);
    let args = vec![
        OsString::from("rev-list"),
        OsString::from("--topo-order"),
        OsString::from(format!("--max-count={requested_count}")),
        OsString::from("HEAD"),
    ];
    let output = run_git_checked("commit-list", args, Some(repository.root()), config).await?;
    let text = output_text(&output, "commit-list")?;
    let mut commit_ids = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        commit_ids.push(validate_commit_id(line)?);
    }
    if commit_ids.is_empty() {
        return Err(GitError::MalformedOutput {
            operation: "commit-list",
        });
    }

    let has_more_commits = commit_ids.len() > options.max_commits;
    if has_more_commits {
        commit_ids.truncate(options.max_commits);
    }

    let mut commits = Vec::with_capacity(commit_ids.len());
    let mut retained_history_paths = 0_usize;
    let mut history_complete = !has_more_commits && !is_shallow;
    for commit_id in &commit_ids {
        let metadata = read_commit_metadata(repository.root(), commit_id, config).await?;
        let remaining_history_paths = options
            .max_total_paths
            .saturating_sub(retained_history_paths);
        let per_commit_limit = options
            .max_changed_paths_per_commit
            .min(remaining_history_paths);
        let (parsed_changes, output_truncated) =
            read_commit_changes(repository.root(), commit_id, per_commit_limit, config).await?;
        let changes_complete = parsed_changes.complete && !output_truncated;
        if !changes_complete {
            history_complete = false;
        }
        let omitted_paths_at_least = if output_truncated || !parsed_changes.complete {
            parsed_changes.omitted_change_count.max(1)
        } else {
            parsed_changes.omitted_change_count
        };
        retained_history_paths =
            retained_history_paths.saturating_add(parsed_changes.changes.len());
        commits.push(GitCommit {
            commit_id: metadata.commit_id,
            parent_commit_ids: metadata.parent_commit_ids,
            committed_at_unix_seconds: metadata.committed_at_unix_seconds,
            author: metadata.author,
            subject: metadata.subject,
            changed_paths: parsed_changes.changes,
            changed_paths_complete: changes_complete,
            omitted_paths_at_least,
        });
    }

    Ok(GitHistory {
        head_commit: repository.head_commit().to_owned(),
        commits,
        complete: history_complete,
        omitted_commits_at_least: if has_more_commits || is_shallow { 1 } else { 0 },
    })
}

async fn repository_is_shallow(root: &Path, config: &GitCommandConfig) -> Result<bool, GitError> {
    let args = vec![
        OsString::from("rev-parse"),
        OsString::from("--is-shallow-repository"),
    ];
    let output = run_git_checked("shallow-state", args, Some(root), config).await?;
    match output_line(&output, "shallow-state")? {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(GitError::MalformedOutput {
            operation: "shallow-state",
        }),
    }
}

struct CommitMetadata {
    commit_id: String,
    parent_commit_ids: Vec<String>,
    committed_at_unix_seconds: i64,
    author: String,
    subject: String,
}

async fn read_commit_metadata(
    root: &Path,
    expected_commit_id: &str,
    config: &GitCommandConfig,
) -> Result<CommitMetadata, GitError> {
    let format = "%H%x00%P%x00%ct%x00%an%x00%s";
    let args = vec![
        OsString::from("show"),
        OsString::from("-s"),
        OsString::from("--no-show-signature"),
        OsString::from(format!("--format={format}")),
        OsString::from(expected_commit_id),
    ];
    let output = run_git_checked("commit-metadata", args, Some(root), config).await?;
    if output.stdout_truncated {
        return Err(GitError::OutputLimitExceeded {
            operation: "commit-metadata",
            stream: "stdout",
        });
    }
    // Git paths and metadata can contain non-UTF-8 bytes. Commit IDs, parents,
    // and timestamps are protocol fields and must parse strictly; display
    // fields use loss replacement instead of failing the whole history scan.
    let fields = output.stdout.split(|byte| *byte == 0).collect::<Vec<_>>();
    if fields.len() != 5 {
        return Err(GitError::MalformedOutput {
            operation: "commit-metadata",
        });
    }
    let commit_id_text = std::str::from_utf8(fields[0]).map_err(|_| GitError::MalformedOutput {
        operation: "commit-metadata",
    })?;
    let commit_id = validate_commit_id(commit_id_text.trim())?;
    if commit_id != expected_commit_id {
        return Err(GitError::MalformedOutput {
            operation: "commit-metadata",
        });
    }
    let parents_text = std::str::from_utf8(fields[1]).map_err(|_| GitError::MalformedOutput {
        operation: "commit-metadata",
    })?;
    let parent_commit_ids = parents_text
        .split_whitespace()
        .map(validate_commit_id)
        .collect::<Result<Vec<_>, _>>()?;
    let timestamp_text = std::str::from_utf8(fields[2]).map_err(|_| GitError::MalformedOutput {
        operation: "commit-metadata",
    })?;
    let committed_at_unix_seconds =
        timestamp_text
            .trim()
            .parse::<i64>()
            .map_err(|_| GitError::MalformedOutput {
                operation: "commit-metadata",
            })?;
    let author = String::from_utf8_lossy(fields[3]).into_owned();
    let subject = String::from_utf8_lossy(fields[4])
        .trim_end_matches(['\r', '\n'])
        .to_owned();
    Ok(CommitMetadata {
        commit_id,
        parent_commit_ids,
        committed_at_unix_seconds,
        author,
        subject,
    })
}

async fn read_commit_changes(
    root: &Path,
    commit_id: &str,
    max_changed_paths: usize,
    config: &GitCommandConfig,
) -> Result<(ParsedPathChanges, bool), GitError> {
    let args = vec![
        OsString::from("diff-tree"),
        OsString::from("--root"),
        OsString::from("--no-commit-id"),
        OsString::from("--name-status"),
        OsString::from("-r"),
        OsString::from("-z"),
        OsString::from("--find-renames"),
        OsString::from("--no-ext-diff"),
        OsString::from("--no-textconv"),
        OsString::from("--diff-merges=first-parent"),
        OsString::from(commit_id),
    ];
    let output = run_git_checked("commit-path-changes", args, Some(root), config).await?;
    let changes = parse_name_status_z(&output.stdout, max_changed_paths);
    Ok((changes, output.stdout_truncated))
}

#[cfg(test)]
mod tests {
    use super::{GitHistoryOptions, MAX_HISTORY_COMMITS};

    #[test]
    fn default_history_bounds_are_finite() {
        let options = GitHistoryOptions::default();
        assert!(options.max_commits > 0);
        assert!(options.max_commits <= MAX_HISTORY_COMMITS);
        assert!(options.max_changed_paths_per_commit > 0);
        assert!(options.max_total_paths >= options.max_changed_paths_per_commit);
    }
}
