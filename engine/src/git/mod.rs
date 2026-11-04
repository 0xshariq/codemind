//! Git mechanics shared by repository lifecycle and analysis engines.
//!
//! This module is deliberately separate from CodeMind's repository lifecycle:
//! `repository/` owns identity, paths, state, and temporary-workspace safety;
//! `git/` obtains trustworthy facts from Git and returns typed results. No
//! caller can provide an arbitrary Git subcommand or execute a shell string.

mod command;
mod diff;
mod history;
mod remote;
mod repository;

pub use command::{GitCommandConfig, GitCommandError};
pub use diff::{GitChangeKind, GitDiff, GitDiffOptions, GitPath, GitPathChange, diff_commits};
pub use history::{GitCommit, GitHistory, GitHistoryOptions, read_history};
pub use remote::{
    RemoteFreshness, RemoteFreshnessObservation, RemoteHead, check_remote_freshness,
    observe_remote_freshness, resolve_remote_head, resolve_remote_ref,
};
pub use repository::{GitRepository, LocalGitStateObservation, open_local_repository};

use thiserror::Error;

use crate::repository::{RepositoryIdentityError, WorkspaceError, WorkspacePhase};

/// Errors surfaced by the typed Git boundary.
#[derive(Debug, Error)]
pub enum GitError {
    #[error(transparent)]
    Command(#[from] GitCommandError),

    #[error(transparent)]
    RepositoryIdentity(#[from] RepositoryIdentityError),

    #[error(transparent)]
    Workspace(#[from] WorkspaceError),

    #[error("Git output for `{operation}` exceeded the configured {stream} capture limit")]
    OutputLimitExceeded {
        operation: &'static str,
        stream: &'static str,
    },

    #[error("Git returned malformed output for `{operation}`")]
    MalformedOutput { operation: &'static str },

    #[error("the supplied path is not a directory: {0:?}")]
    RepositoryPathNotDirectory(std::path::PathBuf),

    #[error("the supplied path is not a Git repository: {0:?}")]
    NotGitRepository(std::path::PathBuf),

    #[error("the supplied path is inside a repository but is not its root: {0:?}")]
    NotRepositoryRoot(std::path::PathBuf),

    #[error("the expected clone destination already exists: {0:?}")]
    CloneDestinationAlreadyExists(std::path::PathBuf),

    #[error(
        "the Git clone is missing expected repository metadata or is not a real directory: {0:?}"
    )]
    InvalidClone(std::path::PathBuf),

    #[error("the workspace repository ID does not match its validated remote locator")]
    RepositoryIdentityMismatch,

    #[error("a credential-free remote Git locator is required")]
    InvalidRemoteLocator,

    #[error("Git reported a different repository root than expected")]
    RepositoryRootMismatch {
        expected: std::path::PathBuf,
        actual: std::path::PathBuf,
    },

    #[error("the workspace is in phase {actual:?}; Git clone requires {expected:?}")]
    WorkspacePhaseMismatch {
        expected: WorkspacePhase,
        actual: WorkspacePhase,
    },

    #[error("invalid Git commit ID; expected 40 or 64 hexadecimal characters")]
    InvalidCommitId,

    #[error("local Git-state fingerprinting is only available for caller-owned repositories")]
    LocalFingerprintRequiresCallerOwnedRepository,

    #[error("the caller-owned repository HEAD changed while its state was being inspected")]
    LocalRepositoryHeadChanged,

    #[error("invalid Git ref")]
    InvalidRef,

    #[error("remote returned conflicting values for the same reference")]
    AmbiguousRemoteRef,

    #[error("Git repository has no matching remote ref `{reference}`")]
    RemoteRefNotFound { reference: String },

    #[error("remote authentication or authorization failed during `{operation}`")]
    RemoteAuthentication { operation: &'static str },

    #[error("remote network access failed during `{operation}`")]
    RemoteNetwork { operation: &'static str },

    #[error("the remote repository could not be found or accessed during `{operation}`")]
    RemoteRepositoryUnavailable { operation: &'static str },

    #[error("Git could not resolve the base commit `{commit}` in the prepared repository")]
    BaseCommitUnavailable { commit: String },

    #[error("Git could not resolve the target commit `{commit}` in the prepared repository")]
    TargetCommitUnavailable { commit: String },

    #[error(
        "base commit `{base_commit}` is not an ancestor of target commit `{target_commit}`; full re-analysis is required"
    )]
    BaseCommitNotAncestor {
        base_commit: String,
        target_commit: String,
    },

    #[error("invalid Git history/diff bounds")]
    InvalidLimits,

    #[error("filesystem operation failed for `{path:?}`: {source}")]
    Filesystem {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Classify remote failures without exposing raw Git stderr or the remote URL.
fn classify_remote_failure(
    operation: &'static str,
    output: &command::GitCommandOutput,
) -> GitError {
    let diagnostic = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if diagnostic.contains("authentication failed")
        || diagnostic.contains("permission denied")
        || diagnostic.contains("could not read username")
        || diagnostic.contains("access denied")
        || diagnostic.contains("unauthorized")
        || diagnostic.contains("forbidden")
    {
        GitError::RemoteAuthentication { operation }
    } else if diagnostic.contains("could not resolve host")
        || diagnostic.contains("failed to connect")
        || diagnostic.contains("connection timed out")
        || diagnostic.contains("network is unreachable")
        || diagnostic.contains("could not resolve proxy")
        || diagnostic.contains("connection refused")
        || diagnostic.contains("operation timed out")
    {
        GitError::RemoteNetwork { operation }
    } else if diagnostic.contains("repository not found")
        || diagnostic.contains("does not appear to be a git repository")
        || diagnostic.contains("could not read from remote repository")
        || diagnostic.contains("repository does not exist")
    {
        GitError::RemoteRepositoryUnavailable { operation }
    } else {
        GitError::Command(GitCommandError::Failed {
            operation,
            status: output.status.code(),
        })
    }
}

pub(super) fn validate_commit_id(value: &str) -> Result<String, GitError> {
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GitError::InvalidCommitId);
    }
    Ok(value.to_ascii_lowercase())
}

pub(super) fn validate_ref(value: &str) -> Result<(), GitError> {
    let invalid = value.trim().is_empty()
        || value.len() > 1_024
        || value.chars().any(char::is_control)
        || value.chars().any(char::is_whitespace)
        || value.contains("..")
        || value == "@"
        || value.contains("@{")
        || value.starts_with('-')
        || value.ends_with('.')
        || value
            .chars()
            .any(|character| matches!(character, '~' | '^' | ':' | '?' | '*' | '[' | '\\'))
        || value.split('/').any(|component| {
            component.is_empty()
                || component.starts_with('.')
                || component.ends_with('.')
                || component.to_ascii_lowercase().ends_with(".lock")
        });
    if invalid {
        return Err(GitError::InvalidRef);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
    };

    pub struct GitFixture {
        _tempdir: tempfile::TempDir,
        pub root: PathBuf,
        /// Commit IDs in chronological order (oldest first).
        pub commits: Vec<String>,
    }

    /// Build a deterministic local repository with three commits: an initial
    /// source tree, a modification plus rename, and a delete/add change.
    pub fn create_three_commit_fixture() -> Result<GitFixture, String> {
        let tempdir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let root = tempdir.path().join("repository");
        fs::create_dir_all(root.join("src")).map_err(|error| error.to_string())?;
        git(&root, &["init", "--quiet"])?;
        git(&root, &["config", "user.name", "CodeMind Test"])?;
        git(
            &root,
            &["config", "user.email", "codemind-tests@example.invalid"],
        )?;
        git(&root, &["config", "commit.gpgsign", "false"])?;

        fs::write(root.join("src/lib.rs"), "pub fn value() -> i32 { 1 }\n")
            .map_err(|error| error.to_string())?;
        fs::write(root.join("src/old_name.rs"), "pub struct OldName;\n")
            .map_err(|error| error.to_string())?;
        git(&root, &["add", "-A"])?;
        git(&root, &["commit", "--quiet", "-m", "initial source"])?;
        let first = git(&root, &["rev-parse", "HEAD"])?;

        fs::write(root.join("src/lib.rs"), "pub fn value() -> i32 { 2 }\n")
            .map_err(|error| error.to_string())?;
        fs::rename(root.join("src/old_name.rs"), root.join("src/new_name.rs"))
            .map_err(|error| error.to_string())?;
        git(&root, &["add", "-A"])?;
        git(&root, &["commit", "--quiet", "-m", "modify and rename"])?;
        let second = git(&root, &["rev-parse", "HEAD"])?;

        fs::remove_file(root.join("src/new_name.rs")).map_err(|error| error.to_string())?;
        fs::write(root.join("src/new.rs"), "pub struct NewType;\n")
            .map_err(|error| error.to_string())?;
        git(&root, &["add", "-A"])?;
        git(&root, &["commit", "--quiet", "-m", "delete and add"])?;
        let third = git(&root, &["rev-parse", "HEAD"])?;

        Ok(GitFixture {
            _tempdir: tempdir,
            root,
            commits: vec![first, second, third],
        })
    }

    fn git(root: &Path, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|error| format!("could not start fixture Git command: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "fixture Git command failed with status {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end_matches(['\r', '\n'])
            .to_owned())
    }
}

#[cfg(test)]
mod level2_tests {
    use super::*;

    #[tokio::test]
    async fn git_modules_compose_history_and_diff_over_one_local_repository() {
        let fixture = test_support::create_three_commit_fixture().unwrap();
        let config = GitCommandConfig::default();
        let repository = open_local_repository(&fixture.root, &config).await.unwrap();
        assert_eq!(repository.head_commit(), fixture.commits.last().unwrap());
        let history = read_history(
            &repository,
            GitHistoryOptions {
                max_commits: 10,
                max_changed_paths_per_commit: 10,
                max_total_paths: 100,
            },
            &config,
        )
        .await
        .unwrap();
        assert_eq!(history.head_commit, fixture.commits[2]);
        assert!(history.commits.len() >= 3);
        let diff = diff_commits(
            &repository,
            &fixture.commits[0],
            &fixture.commits[2],
            GitDiffOptions::default(),
            &config,
        )
        .await
        .unwrap();
        assert!(
            diff.changes
                .iter()
                .any(|change| matches!(change.kind, GitChangeKind::Modified))
        );
        assert!(
            diff.changes
                .iter()
                .any(|change| matches!(change.kind, GitChangeKind::Added | GitChangeKind::Renamed))
        );
    }
}

#[cfg(test)]
mod history_level2_tests {
    use super::*;

    #[tokio::test]
    async fn reads_three_commit_lineage_and_rename_history_locally() {
        let fixture =
            test_support::create_three_commit_fixture().expect("create local Git fixture");
        let repository = open_local_repository(&fixture.root, &GitCommandConfig::default())
            .await
            .expect("open local Git fixture");
        let history = read_history(
            &repository,
            GitHistoryOptions {
                max_commits: 10,
                max_changed_paths_per_commit: 100,
                max_total_paths: 1_000,
            },
            &GitCommandConfig::default(),
        )
        .await
        .expect("read Git history");

        assert!(history.complete);
        assert_eq!(history.commits.len(), 3);
        assert_eq!(history.head_commit, fixture.commits[2]);
        assert_eq!(history.commits[0].commit_id, fixture.commits[2]);
        assert_eq!(
            history.commits[0].parent_commit_ids,
            vec![fixture.commits[1].clone()]
        );
        assert!(
            history.commits[1]
                .changed_paths
                .iter()
                .any(|change| change.kind == GitChangeKind::Renamed)
        );
    }
}

#[cfg(test)]
mod diff_level2_tests {
    use super::*;

    #[tokio::test]
    async fn same_commit_produces_an_empty_complete_diff_with_repository_provenance() {
        let fixture =
            test_support::create_three_commit_fixture().expect("create local Git fixture");
        let repository = open_local_repository(&fixture.root, &GitCommandConfig::default())
            .await
            .expect("open local Git fixture");
        let diff = diff_commits(
            &repository,
            &fixture.commits[2],
            &fixture.commits[2],
            GitDiffOptions::default(),
            &GitCommandConfig::default(),
        )
        .await
        .expect("compute empty commit diff");

        assert!(diff.complete);
        assert!(diff.changes.is_empty());
        assert_eq!(diff.omitted_change_count, 0);
        assert!(!diff.repository_id.is_empty());
        assert_eq!(diff.base_commit, diff.target_commit);
    }

    #[tokio::test]
    async fn missing_base_commit_requests_safe_full_reanalysis() {
        let fixture =
            test_support::create_three_commit_fixture().expect("create local Git fixture");
        let repository = open_local_repository(&fixture.root, &GitCommandConfig::default())
            .await
            .expect("open local Git fixture");
        let result = diff_commits(
            &repository,
            &"0".repeat(40),
            repository.head_commit(),
            GitDiffOptions::default(),
            &GitCommandConfig::default(),
        )
        .await;

        assert!(matches!(
            result,
            Err(GitError::BaseCommitUnavailable { .. })
        ));
    }

    #[tokio::test]
    async fn computes_a_bounded_diff_between_local_commits() {
        let fixture =
            test_support::create_three_commit_fixture().expect("create local Git fixture");
        let repository = open_local_repository(&fixture.root, &GitCommandConfig::default())
            .await
            .expect("open local Git fixture");
        let diff = diff_commits(
            &repository,
            &fixture.commits[0],
            &fixture.commits[2],
            GitDiffOptions {
                max_changed_paths: 100,
            },
            &GitCommandConfig::default(),
        )
        .await
        .expect("compute commit diff");

        assert!(diff.complete);
        assert_eq!(diff.base_commit, fixture.commits[0]);
        assert_eq!(diff.target_commit, fixture.commits[2]);
        assert!(diff.changes.iter().any(|change| {
            change.kind == GitChangeKind::Added
                && change
                    .new_path
                    .as_ref()
                    .is_some_and(|path| path.display == "src/new.rs")
        }));
        assert!(diff.changes.iter().any(|change| {
            change.kind == GitChangeKind::Deleted
                && change
                    .old_path
                    .as_ref()
                    .is_some_and(|path| path.display == "src/old_name.rs")
        }));
    }

    #[tokio::test]
    async fn rejects_a_base_commit_from_a_divergent_branch() {
        use std::{fs, process::Command};

        let fixture =
            test_support::create_three_commit_fixture().expect("create local Git fixture");
        let run_git = |args: &[&str]| {
            let output = Command::new("git")
                .args(args)
                .current_dir(&fixture.root)
                .output()
                .expect("run local Git command");
            assert!(
                output.status.success(),
                "Git command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout)
                .trim_end_matches(['\r', '\n'])
                .to_owned()
        };
        run_git(&[
            "checkout",
            "--quiet",
            "-b",
            "side",
            fixture.commits[0].as_str(),
        ]);
        fs::write(
            fixture.root.join("src/lib.rs"),
            "pub fn side() -> i32 { 9 }\n",
        )
        .expect("write divergent source");
        run_git(&["add", "-A"]);
        run_git(&["commit", "--quiet", "-m", "divergent change"]);
        let divergent_head = run_git(&["rev-parse", "HEAD"]);
        let repository = open_local_repository(&fixture.root, &GitCommandConfig::default())
            .await
            .expect("open divergent local repository");

        let result = diff_commits(
            &repository,
            &fixture.commits[1],
            &divergent_head,
            GitDiffOptions {
                max_changed_paths: 100,
            },
            &GitCommandConfig::default(),
        )
        .await;

        assert!(matches!(
            result,
            Err(GitError::BaseCommitNotAncestor { .. })
        ));
    }
}

#[cfg(test)]
mod repository_level2_tests {
    use super::*;

    #[tokio::test]
    async fn opens_a_local_repository_without_taking_ownership() {
        let fixture =
            test_support::create_three_commit_fixture().expect("create local Git fixture");
        let repository = open_local_repository(&fixture.root, &GitCommandConfig::default())
            .await
            .expect("open local Git fixture");
        assert_eq!(repository.head_commit(), fixture.commits[2].as_str());
        assert!(!repository.is_owned_temporary_clone());
        assert!(fixture.root.join(".git").is_dir());
    }
}

#[cfg(test)]
mod local_freshness_level2_tests {
    use super::*;
    use std::{fs, process::Command};

    fn git(root: &std::path::Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("run fixture Git command");
        assert!(
            output.status.success(),
            "Git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[tokio::test]
    async fn local_fingerprint_detects_index_and_worktree_changes_without_mutating_checkout() {
        let fixture =
            test_support::create_three_commit_fixture().expect("create local Git fixture");
        let repository = open_local_repository(&fixture.root, &GitCommandConfig::default())
            .await
            .expect("open caller-owned repository");
        let config = GitCommandConfig::default();
        let initial_observation = repository
            .local_state_observation(&config)
            .await
            .expect("observe clean checkout");
        assert!(initial_observation.working_tree_clean);
        let initial = initial_observation.digest;
        let repeated = repository
            .local_state_observation(&config)
            .await
            .expect("repeat clean checkout observation");
        assert!(repeated.working_tree_clean);
        assert_eq!(initial, repeated.digest);
        let original_head = repository.head_commit().to_owned();
        let index_path = fixture.root.join(".git/index");
        let initial_index = fs::read(&index_path).expect("read Git index");

        // Stage content different from HEAD, then restore the working tree to
        // HEAD. The source manifest alone would match the old bytes, but the
        // staged index must still invalidate reuse.
        fs::write(
            fixture.root.join("src/lib.rs"),
            "pub fn value() -> i32 { 99 }\n",
        )
        .unwrap();
        git(&fixture.root, &["add", "src/lib.rs"]);
        fs::write(
            fixture.root.join("src/lib.rs"),
            "pub fn value() -> i32 { 2 }\n",
        )
        .unwrap();
        let staged_observation = repository
            .local_state_observation(&config)
            .await
            .expect("observe staged index change");
        assert!(!staged_observation.working_tree_clean);
        let staged = staged_observation.digest;
        assert_ne!(initial, staged);

        let staged_index = fs::read(&index_path).expect("read staged Git index");
        assert_ne!(initial_index, staged_index);
        assert_eq!(repository.head_commit(), original_head);
        assert_eq!(
            fs::read_to_string(fixture.root.join("src/lib.rs")).unwrap(),
            "pub fn value() -> i32 { 2 }\n"
        );
        assert!(fixture.root.join(".git").is_dir());

        // An untracked file is also part of the Git-state fingerprint.
        fs::write(fixture.root.join("untracked.txt"), "not indexed\n").unwrap();
        let untracked_observation = repository
            .local_state_observation(&config)
            .await
            .expect("observe untracked file");
        assert!(!untracked_observation.working_tree_clean);
        let with_untracked = untracked_observation.digest;
        assert_ne!(staged, with_untracked);

        // Fingerprinting is read-only: status must not refresh/write the index.
        assert_eq!(
            staged_index,
            fs::read(&index_path).expect("read Git index after fingerprint")
        );
        assert!(fixture.root.join("untracked.txt").is_file());
    }

    #[tokio::test]
    async fn local_fingerprint_refuses_a_head_that_changed_after_open() {
        let fixture =
            test_support::create_three_commit_fixture().expect("create local Git fixture");
        let repository = open_local_repository(&fixture.root, &GitCommandConfig::default())
            .await
            .expect("open caller-owned repository");
        fs::write(
            fixture.root.join("src/lib.rs"),
            "pub fn value() -> i32 { 777 }\n",
        )
        .unwrap();
        git(&fixture.root, &["add", "src/lib.rs"]);
        git(
            &fixture.root,
            &["commit", "--quiet", "-m", "advance after open"],
        );

        let error = repository
            .local_state_fingerprint(&GitCommandConfig::default())
            .await
            .expect_err("a cached GitRepository must not hide a changed HEAD");
        assert!(matches!(error, GitError::LocalRepositoryHeadChanged));
        assert!(fixture.root.join(".git").is_dir());
        assert!(fixture.root.join("src/lib.rs").is_file());
    }

    #[tokio::test]
    async fn local_fingerprint_covers_unstaged_changes_and_staged_rename_status() {
        let fixture =
            test_support::create_three_commit_fixture().expect("create local Git fixture");
        let repository = open_local_repository(&fixture.root, &GitCommandConfig::default())
            .await
            .expect("open caller-owned repository");
        let config = GitCommandConfig::default();
        let initial = repository.local_state_fingerprint(&config).await.unwrap();

        fs::write(
            fixture.root.join("src/lib.rs"),
            "pub fn value() -> i32 { 123 }\n",
        )
        .unwrap();
        let modified_observation = repository.local_state_observation(&config).await.unwrap();
        assert!(!modified_observation.working_tree_clean);
        let modified = modified_observation.digest;
        assert_ne!(initial, modified);

        fs::remove_file(fixture.root.join("src/new.rs")).unwrap();
        let deleted_observation = repository.local_state_observation(&config).await.unwrap();
        assert!(!deleted_observation.working_tree_clean);
        let deleted = deleted_observation.digest;
        assert_ne!(modified, deleted);

        fs::write(fixture.root.join("src/new.rs"), "pub struct NewType;\n").unwrap();
        let restored = repository.local_state_fingerprint(&config).await.unwrap();
        assert_ne!(deleted, restored);

        git(&fixture.root, &["mv", "src/new.rs", "src/renamed.rs"]);
        let renamed_observation = repository.local_state_observation(&config).await.unwrap();
        assert!(!renamed_observation.working_tree_clean);
        assert_ne!(restored, renamed_observation.digest);
        assert!(fixture.root.join(".git").is_dir());
        assert!(fixture.root.join("src/lib.rs").is_file());
    }
}

#[cfg(test)]
mod remote_level2_tests {
    use super::remote::{is_unknown_remote_state, parse_requested_ref_output};
    use super::*;

    #[test]
    fn inaccessible_invalid_and_ambiguous_remote_results_are_unknown() {
        assert!(is_unknown_remote_state(&GitError::RemoteAuthentication {
            operation: "remote-ref",
        }));
        assert!(is_unknown_remote_state(&GitError::RemoteNetwork {
            operation: "remote-ref",
        }));
        assert!(is_unknown_remote_state(
            &GitError::RemoteRepositoryUnavailable {
                operation: "remote-ref",
            }
        ));
        assert!(is_unknown_remote_state(&GitError::RemoteRefNotFound {
            reference: "main".to_owned(),
        }));
        assert!(is_unknown_remote_state(&GitError::InvalidRef));
        assert!(is_unknown_remote_state(&GitError::InvalidRemoteLocator));
        assert!(is_unknown_remote_state(&GitError::InvalidCommitId));
        assert!(is_unknown_remote_state(&GitError::AmbiguousRemoteRef));
        assert_ne!(RemoteFreshness::Unknown, RemoteFreshness::Unchanged);
    }

    #[test]
    fn conflicting_advertisements_for_one_remote_ref_are_rejected() {
        let first = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let second = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let output = format!("{first}\trefs/heads/main\n{second}\trefs/heads/main\n");
        let error = parse_requested_ref_output(
            &output,
            "main",
            Some("refs/heads/main"),
            Some("refs/tags/main"),
        )
        .expect_err("conflicting advertisements must not select either commit");
        assert!(matches!(error, GitError::AmbiguousRemoteRef));
        assert!(is_unknown_remote_state(&error));
    }

    #[test]
    fn annotated_tag_prefers_peeled_commit_hash() {
        let tag_object = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let commit = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let output = format!("{tag_object}\trefs/tags/v1\n{commit}\trefs/tags/v1^{{}}\n");
        let (reference, commit_id) =
            parse_requested_ref_output(&output, "v1", None, Some("refs/tags/v1"))
                .expect("parse annotated tag");
        assert_eq!(reference, "refs/tags/v1");
        assert_eq!(commit_id, commit);
    }

    #[test]
    fn short_ref_prefers_branch_when_branch_and_tag_share_name() {
        let branch_commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let tag_commit = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let output = format!("{branch_commit}\trefs/heads/v1\n{tag_commit}\trefs/tags/v1\n");
        let (reference, commit_id) =
            parse_requested_ref_output(&output, "v1", Some("refs/heads/v1"), Some("refs/tags/v1"))
                .expect("parse branch and tag");
        assert_eq!(reference, "refs/heads/v1");
        assert_eq!(commit_id, branch_commit);
    }

    #[tokio::test]
    async fn unsupported_file_locator_is_rejected_before_process_start() {
        let result = resolve_remote_ref(
            "file:///tmp/not-a-supported-remote.git",
            "main",
            &GitCommandConfig::default(),
        )
        .await;
        let error = result.expect_err("unsupported file locator must fail");
        assert!(error.to_string().contains("unsupported"));
    }
}
