//! Repository-level identity, durable state, and temporary workspace lifecycle.
//!
//! The repository boundary owns the lifecycle around source acquisition. The
//! separate `git` module owns Git command and history mechanics, while the
//! existing `source` module remains responsible for immutable source snapshots.

mod identity;
mod manager;
mod paths;
mod recovery;
mod state;
mod workspace;

pub use identity::{
    REPOSITORY_METADATA_SCHEMA_VERSION, RepositoryIdentity, RepositoryIdentityError,
    RepositoryIdentityKind, RepositoryMetadata, workspace_scope_key,
};
pub use manager::{
    PreparedRepository, RepositoryKnowledge, RepositoryManager, RepositoryManagerError,
    RepositoryManagerOptions,
};
pub use paths::{CodeMindPaths, CodeMindPathsError, RepositoryScopeLock};
pub use recovery::{
    WorkspaceRecovery, WorkspaceRecoveryError, WorkspaceRecoveryIssue, WorkspaceRecoveryReport,
};
pub use state::{
    AnalysisState, CleanupState, GraphVerificationReceipt, KnowledgeCompleteness,
    REPOSITORY_STATE_SCHEMA_VERSION, RemoteFreshnessState, RepositoryGraphState, RepositoryState,
    RepositoryStateError,
};
pub use workspace::{
    WORKSPACE_MARKER_SCHEMA_VERSION, WorkspaceCleanupReason, WorkspaceError, WorkspaceManager,
    WorkspaceMarker, WorkspacePhase, WorkspaceRetryMode,
};

#[cfg(test)]
mod level2_tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn repository_modules_compose_stable_identity_state_and_workspace_contracts() {
        let identity =
            RepositoryIdentity::from_remote("https://example.invalid/owner/CodeMind.git/").unwrap();
        assert!(identity.repository_id().starts_with("repo-"));

        let legacy_scope = workspace_scope_key("workspace-1").unwrap();
        let legacy_state =
            RepositoryState::new(identity.repository_id().to_owned(), legacy_scope.clone())
                .unwrap();
        assert_eq!(legacy_state.repository_id, identity.repository_id());
        assert_eq!(legacy_state.workspace_scope_key, legacy_scope);
        assert_eq!(legacy_state.workspace_id, legacy_scope);

        let workspace_id = "workspace-real";
        let workspace_scope = workspace_scope_key(workspace_id).unwrap();
        let scoped = RepositoryState::new_for_workspace(
            identity.repository_id().to_owned(),
            workspace_id,
            workspace_scope.clone(),
        )
        .unwrap();
        assert_eq!(scoped.workspace_id, workspace_id);
        assert_eq!(scoped.workspace_scope_key, workspace_scope);
        assert!(scoped.graph_state().unwrap().is_none());

        let metadata = RepositoryMetadata::new(identity.clone()).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let paths = CodeMindPaths::from_home_dir(temp.path()).unwrap();
        metadata.save(&paths).unwrap();
        assert_eq!(
            RepositoryMetadata::load(&paths, identity.repository_id()).unwrap(),
            metadata
        );

        let manager = WorkspaceManager::new(paths);
        let marker = manager
            .create(
                "op_repository_contract",
                &identity,
                &workspace_scope,
                Some("main"),
            )
            .unwrap();
        assert_eq!(marker.phase, WorkspacePhase::Preparing);
        assert_eq!(marker.repository_id.as_str(), identity.repository_id());
        assert_eq!(
            marker.workspace_scope_key.as_str(),
            workspace_scope.as_str()
        );
        let reloaded_marker = manager.load_marker("op_repository_contract").unwrap();
        assert!(reloaded_marker.workspace_root.is_dir());
    }

    #[test]
    fn codemind_checkout_persists_repository_state_under_the_home_dot_codemind_root() {
        // The engine package is expected to be checked out at
        // <codemind>/engine. The test uses that real working tree, while all
        // writes are isolated to a temporary home directory.
        let engine_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repository_root = engine_root
            .parent()
            .expect("the CodeMind engine must live below the repository root");
        assert!(
            repository_root.join("README.md").is_file()
                && repository_root.join("engine/Cargo.toml").is_file()
                && repository_root.join(".git").exists(),
            "this test must run from the CodeMind GitHub checkout"
        );

        let origin_output = Command::new("git")
            .arg("-C")
            .arg(repository_root)
            .args(["remote", "get-url", "origin"])
            .output()
            .expect("read the CodeMind checkout's GitHub origin");
        assert!(
            origin_output.status.success(),
            "the CodeMind checkout must have an origin remote: {}",
            String::from_utf8_lossy(&origin_output.stderr)
        );
        let origin = String::from_utf8(origin_output.stdout).expect("Git origin must be UTF-8");
        let origin = origin.trim();
        let repo_local_state_before = repository_root.join(".codemind").exists();
        let identity =
            RepositoryIdentity::from_local_directory_with_origin(repository_root, Some(origin))
                .expect("derive CodeMind identity from its actual local checkout and origin");
        assert!(
            matches!(
                identity.canonical_locator(),
                "https://github.com/0xshariq/codemind" | "ssh://git@github.com/0xshariq/codemind"
            ),
            "unexpected CodeMind origin identity: {}",
            identity.canonical_locator()
        );
        assert_eq!(identity.kind(), RepositoryIdentityKind::Remote);

        let temp_home = tempfile::tempdir().expect("create isolated test home");
        let paths =
            CodeMindPaths::from_home_dir(temp_home.path()).expect("resolve .codemind paths");
        paths
            .ensure_base_dirs()
            .expect("create private .codemind layout");
        let expected_codemind_root = temp_home.path().join(".codemind");
        assert_eq!(paths.root(), expected_codemind_root.as_path());
        assert!(paths.repositories_dir().is_dir());
        assert!(paths.workspaces_dir().is_dir());
        assert!(paths.locks_dir().is_dir());

        let metadata =
            RepositoryMetadata::new(identity.clone()).expect("construct repository metadata");
        metadata
            .save(&paths)
            .expect("persist CodeMind repository metadata");
        let loaded_metadata = RepositoryMetadata::load(&paths, identity.repository_id())
            .expect("reload CodeMind repository metadata");
        assert_eq!(
            loaded_metadata.identity.repository_id(),
            identity.repository_id()
        );
        assert_eq!(
            loaded_metadata.identity.canonical_locator(),
            identity.canonical_locator()
        );
        let metadata_path = paths
            .repository_metadata_path(identity.repository_id())
            .expect("resolve metadata record path");
        assert!(metadata_path.is_file());

        let workspace_id = "codemind-repository-level2-test";
        let workspace_scope =
            workspace_scope_key(workspace_id).expect("derive workspace security scope");
        let state = RepositoryState::new_for_workspace(
            identity.repository_id().to_owned(),
            workspace_id,
            workspace_scope.clone(),
        )
        .expect("construct repository state with distinct workspace identity");
        state.save(&paths).expect("persist scoped repository state");
        let loaded_state =
            RepositoryState::load(&paths, identity.repository_id(), &workspace_scope)
                .expect("reload scoped repository state");
        assert_eq!(
            loaded_state.repository_id.as_str(),
            identity.repository_id()
        );
        assert_eq!(loaded_state.workspace_id.as_str(), workspace_id);
        assert_eq!(
            loaded_state.workspace_scope_key.as_str(),
            workspace_scope.as_str()
        );
        let state_path = paths
            .repository_state_path(identity.repository_id(), &workspace_scope)
            .expect("resolve scoped state record path");
        assert!(state_path.is_file());

        let workspace_manager = WorkspaceManager::new(paths.clone());
        let marker = workspace_manager
            .create(
                "op_codemind_repo_level2",
                &identity,
                &workspace_scope,
                Some("main"),
            )
            .expect("create an operation-owned CodeMind workspace marker");
        let reloaded_marker = workspace_manager
            .load_marker("op_codemind_repo_level2")
            .expect("reload CodeMind workspace marker");
        assert_eq!(
            reloaded_marker.repository_id.as_str(),
            identity.repository_id()
        );
        assert_eq!(
            reloaded_marker.workspace_scope_key.as_str(),
            workspace_scope.as_str()
        );
        assert_eq!(reloaded_marker.phase, WorkspacePhase::Preparing);
        assert!(marker.workspace_root.is_dir());
        assert!(marker.workspace_root.join("workspace.json").is_file());
        assert_eq!(
            marker.workspace_root.join("workspace.json"),
            paths
                .workspace_marker_path("op_codemind_repo_level2")
                .expect("resolve workspace marker path")
        );
        assert!(!marker.repository_root.exists());
        assert!(!paths.config_dir().exists());
        assert!(
            std::fs::read_dir(paths.workspaces_dir())
                .expect("read operation workspace directory")
                .next()
                .is_some(),
            "the owned workspace marker must be stored under .codemind/workspaces"
        );

        // Durable engine state belongs below the configured home directory,
        // never inside the source checkout being analyzed.
        assert_eq!(
            repository_root.join(".codemind").exists(),
            repo_local_state_before,
            "repository resolution must not create or alter a repo-local .codemind directory"
        );
    }
}

#[cfg(test)]
mod workspace_level2_tests {
    use super::*;

    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    fn manager() -> (TempDir, WorkspaceManager, RepositoryIdentity, String) {
        let home = TempDir::new().unwrap();
        let paths = CodeMindPaths::from_home_dir(home.path()).unwrap();
        let manager = WorkspaceManager::new(paths);
        let identity =
            RepositoryIdentity::from_remote("https://github.com/owner/repo.git").unwrap();
        let scope = workspace_scope_key("workspace-123").unwrap();
        (home, manager, identity, scope)
    }

    #[test]
    fn creates_marker_before_checkout_and_rejects_reusing_operation_id() {
        let (_home, manager, identity, scope) = manager();
        let marker = manager
            .create("op_123", &identity, &scope, Some("main"))
            .unwrap();
        assert_eq!(marker.phase, WorkspacePhase::Preparing);
        assert!(marker.workspace_root.is_dir());
        assert!(marker.workspace_root.join("workspace.json").is_file());
        assert!(!marker.repository_root.exists());
        assert!(matches!(
            manager.create("op_123", &identity, &scope, Some("main")),
            Err(WorkspaceError::WorkspaceAlreadyExists(_))
        ));
    }

    #[test]
    fn lifecycle_records_head_and_rejects_skipped_transitions() {
        let (_home, manager, identity, scope) = manager();
        manager
            .create("op_1", &identity, &scope, Some("main"))
            .unwrap();
        assert!(matches!(
            manager.transition("op_1", WorkspacePhase::Analyzing),
            Err(WorkspaceError::InvalidTransition { .. })
        ));
        manager.transition("op_1", WorkspacePhase::Cloning).unwrap();
        let marker = manager.record_head_commit("op_1", &"A".repeat(40)).unwrap();
        let expected_head = "a".repeat(40);
        assert_eq!(marker.head_commit.as_deref(), Some(expected_head.as_str()));
        manager
            .transition("op_1", WorkspacePhase::Analyzing)
            .unwrap();
        assert_eq!(
            manager.load_marker("op_1").unwrap().phase,
            WorkspacePhase::Analyzing
        );
    }

    #[test]
    fn cleanup_removes_the_entire_owned_checkout_including_git() {
        let (_home, manager, identity, scope) = manager();
        manager
            .create("op_cleanup", &identity, &scope, Some("main"))
            .unwrap();
        manager
            .transition("op_cleanup", WorkspacePhase::Cloning)
            .unwrap();
        let checkout = manager
            .paths()
            .workspace_repository_dir("op_cleanup")
            .unwrap();
        fs::create_dir_all(checkout.join(".git/objects")).unwrap();
        fs::create_dir_all(checkout.join("src")).unwrap();
        fs::write(checkout.join(".git/objects/test-object"), b"git object").unwrap();
        fs::write(checkout.join("src/lib.rs"), b"fn test() {}").unwrap();
        manager
            .record_head_commit("op_cleanup", &"b".repeat(40))
            .unwrap();

        manager
            .transition("op_cleanup", WorkspacePhase::Analyzing)
            .unwrap();
        manager
            .transition("op_cleanup", WorkspacePhase::Persisting)
            .unwrap();
        manager
            .transition("op_cleanup", WorkspacePhase::Verifying)
            .unwrap();
        manager
            .transition("op_cleanup", WorkspacePhase::Verified)
            .unwrap();
        assert_eq!(
            manager.load_marker("op_cleanup").unwrap().phase,
            WorkspacePhase::Verified
        );
        manager.cleanup_after_verification("op_cleanup").unwrap();
        assert!(
            !manager
                .paths()
                .workspace_dir("op_cleanup")
                .unwrap()
                .exists()
        );
    }

    #[test]
    fn cleanup_requires_the_full_verified_workspace_lifecycle() {
        let (_home, manager, identity, scope) = manager();
        manager
            .create("op_lifecycle", &identity, &scope, None)
            .unwrap();
        manager
            .transition("op_lifecycle", WorkspacePhase::Cloning)
            .unwrap();
        manager
            .record_head_commit("op_lifecycle", &"d".repeat(40))
            .unwrap();
        manager
            .transition("op_lifecycle", WorkspacePhase::Analyzing)
            .unwrap();
        manager
            .transition("op_lifecycle", WorkspacePhase::Persisting)
            .unwrap();
        manager
            .transition("op_lifecycle", WorkspacePhase::Verifying)
            .unwrap();

        assert!(matches!(
            manager.cleanup_after_verification("op_lifecycle"),
            Err(WorkspaceError::VerificationRequiredForCleanup)
        ));
        assert!(
            manager
                .paths()
                .workspace_dir("op_lifecycle")
                .unwrap()
                .is_dir()
        );

        manager
            .transition("op_lifecycle", WorkspacePhase::Verified)
            .unwrap();
        manager.cleanup_after_verification("op_lifecycle").unwrap();
        assert!(
            !manager
                .paths()
                .workspace_dir("op_lifecycle")
                .unwrap()
                .exists()
        );
    }

    #[test]
    fn successful_path_cleanup_requires_verified_graph_state() {
        let (_home, manager, identity, scope) = manager();
        manager
            .create("op_unverified", &identity, &scope, None)
            .unwrap();
        assert!(matches!(
            manager.cleanup_after_verification("op_unverified"),
            Err(WorkspaceError::VerificationRequiredForCleanup)
        ));
        assert!(
            manager
                .paths()
                .workspace_dir("op_unverified")
                .unwrap()
                .is_dir()
        );
    }

    #[test]
    fn missing_or_corrupt_marker_never_authorizes_deletion() {
        let (_home, manager, identity, scope) = manager();
        manager
            .create("op_marker", &identity, &scope, None)
            .unwrap();
        let marker_path = manager.paths().workspace_marker_path("op_marker").unwrap();
        fs::remove_file(&marker_path).unwrap();
        assert!(matches!(
            manager.cleanup_after_verification("op_marker"),
            Err(WorkspaceError::MarkerMissing)
        ));
        assert!(manager.paths().workspace_dir("op_marker").unwrap().is_dir());

        fs::write(&marker_path, b"not json").unwrap();
        assert!(manager.cleanup_after_verification("op_marker").is_err());
        assert!(manager.paths().workspace_dir("op_marker").unwrap().is_dir());
    }

    #[test]
    fn tampered_marker_paths_cannot_redirect_cleanup_outside_workspace_root() {
        let (home, manager, identity, scope) = manager();
        manager
            .create("op_tamper", &identity, &scope, None)
            .unwrap();
        let outside = home.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep.txt"), b"keep").unwrap();

        let marker_path = manager.paths().workspace_marker_path("op_tamper").unwrap();
        let mut marker: serde_json::Value =
            serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
        marker["workspace_root"] =
            serde_json::Value::String(outside.to_string_lossy().into_owned());
        marker["repository_root"] =
            serde_json::Value::String(outside.join("repository").to_string_lossy().into_owned());
        fs::write(&marker_path, serde_json::to_vec_pretty(&marker).unwrap()).unwrap();

        assert!(matches!(
            manager.cleanup_after_verification("op_tamper"),
            Err(WorkspaceError::MarkerPathMismatch)
        ));
        assert!(outside.join("keep.txt").is_file());
        assert!(manager.paths().workspace_dir("op_tamper").unwrap().is_dir());
    }

    #[test]
    fn failed_operation_retains_owned_clone_and_successful_retry_cleans_it() {
        let (_home, manager, identity, scope) = manager();
        manager
            .create("op_failed", &identity, &scope, None)
            .unwrap();
        let checkout = manager
            .paths()
            .workspace_repository_dir("op_failed")
            .unwrap();
        fs::create_dir_all(checkout.join(".git/objects")).unwrap();
        fs::create_dir_all(checkout.join("src")).unwrap();
        fs::write(checkout.join("src/lib.rs"), b"pub fn retained() {}").unwrap();

        manager
            .transition("op_failed", WorkspacePhase::Cloning)
            .unwrap();
        manager
            .record_head_commit("op_failed", &"d".repeat(40))
            .unwrap();
        manager
            .transition("op_failed", WorkspacePhase::Analyzing)
            .unwrap();
        manager
            .mark_retryable_failure("op_failed", "graph_verification_failed")
            .unwrap();

        let marker = manager.load_marker("op_failed").unwrap();
        assert_eq!(marker.phase, WorkspacePhase::RetryableFailure);
        assert_eq!(
            marker.failure_code.as_deref(),
            Some("graph_verification_failed")
        );
        assert!(checkout.join(".git/objects").is_dir());
        assert!(checkout.join("src/lib.rs").is_file());

        assert_eq!(
            manager.begin_retry("op_failed").unwrap(),
            WorkspaceRetryMode::ResumeExistingClone
        );
        manager.resume_analysis("op_failed").unwrap();
        manager
            .transition("op_failed", WorkspacePhase::Persisting)
            .unwrap();
        manager
            .transition("op_failed", WorkspacePhase::Verifying)
            .unwrap();
        manager
            .transition("op_failed", WorkspacePhase::Verified)
            .unwrap();
        manager.cleanup_after_verification("op_failed").unwrap();

        assert!(!manager.paths().workspace_dir("op_failed").unwrap().exists());
    }

    #[test]
    fn retry_reclones_when_the_owned_checkout_is_missing() {
        let (_home, manager, identity, scope) = manager();
        manager
            .create("op_missing_checkout", &identity, &scope, None)
            .unwrap();
        manager
            .transition("op_missing_checkout", WorkspacePhase::Cloning)
            .unwrap();
        manager
            .record_head_commit("op_missing_checkout", &"f".repeat(40))
            .unwrap();
        manager
            .transition("op_missing_checkout", WorkspacePhase::Analyzing)
            .unwrap();
        manager
            .mark_retryable_failure("op_missing_checkout", "checkout_lost")
            .unwrap();

        let checkout = manager
            .paths()
            .workspace_repository_dir("op_missing_checkout")
            .unwrap();
        fs::create_dir_all(&checkout).unwrap();
        fs::remove_dir_all(&checkout).unwrap();

        assert_eq!(
            manager.begin_retry("op_missing_checkout").unwrap(),
            WorkspaceRetryMode::CloneAgain
        );
        let marker = manager.load_marker("op_missing_checkout").unwrap();
        assert_eq!(marker.phase, WorkspacePhase::Preparing);
        assert!(marker.head_commit.is_none());
        assert!(!checkout.exists());
    }

    #[cfg(unix)]
    #[test]
    fn retry_rejects_symlinked_git_metadata_without_touching_its_target() {
        let (home, manager, identity, scope) = manager();
        manager
            .create("op_symlink_git", &identity, &scope, None)
            .unwrap();
        manager
            .transition("op_symlink_git", WorkspacePhase::Cloning)
            .unwrap();
        let checkout = manager
            .paths()
            .workspace_repository_dir("op_symlink_git")
            .unwrap();
        fs::create_dir_all(&checkout).unwrap();
        let outside = home.path().join("outside-git");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep"), b"keep").unwrap();
        symlink(&outside, checkout.join(".git")).unwrap();

        manager
            .record_head_commit("op_symlink_git", &"1".repeat(40))
            .unwrap();
        manager
            .transition("op_symlink_git", WorkspacePhase::Analyzing)
            .unwrap();
        manager
            .mark_retryable_failure("op_symlink_git", "interrupted")
            .unwrap();

        assert!(matches!(
            manager.begin_retry("op_symlink_git"),
            Err(WorkspaceError::UnsafeWorkspacePath(_))
        ));
        assert!(outside.join("keep").is_file());
        assert!(checkout.join(".git").is_symlink());
    }

    #[test]
    fn cleanup_failure_keeps_the_marker_and_can_be_retried_safely() {
        let (_home, manager, identity, scope) = manager();
        let marker = manager
            .create("op_cleanup_retry", &identity, &scope, None)
            .unwrap();
        manager
            .transition("op_cleanup_retry", WorkspacePhase::Cloning)
            .unwrap();
        manager
            .record_head_commit("op_cleanup_retry", &"d".repeat(40))
            .unwrap();
        manager
            .transition("op_cleanup_retry", WorkspacePhase::Analyzing)
            .unwrap();
        manager
            .transition("op_cleanup_retry", WorkspacePhase::Persisting)
            .unwrap();
        manager
            .transition("op_cleanup_retry", WorkspacePhase::Verifying)
            .unwrap();
        manager
            .transition("op_cleanup_retry", WorkspacePhase::Verified)
            .unwrap();

        let unexpected = marker.workspace_root.join("unexpected.txt");
        fs::write(&unexpected, b"preserve until ownership is clear").unwrap();
        assert!(matches!(
            manager.cleanup_after_verification("op_cleanup_retry"),
            Err(WorkspaceError::UnexpectedWorkspaceEntry(path)) if path == unexpected
        ));

        let retained = manager.load_marker("op_cleanup_retry").unwrap();
        assert_eq!(retained.phase, WorkspacePhase::CleanupFailed);
        assert_eq!(
            retained.cleanup_reason,
            Some(WorkspaceCleanupReason::GraphsVerified)
        );
        assert!(unexpected.is_file());

        fs::remove_file(&unexpected).unwrap();
        manager
            .cleanup_after_verification("op_cleanup_retry")
            .unwrap();
        assert!(!marker.workspace_root.exists());
    }

    #[test]
    fn retryable_workspace_failure_recording_is_idempotent_and_non_destructive() {
        let (_home, manager, identity, scope) = manager();
        manager
            .create("op_retry_idempotent", &identity, &scope, None)
            .unwrap();
        manager
            .transition("op_retry_idempotent", WorkspacePhase::Cloning)
            .unwrap();
        manager
            .record_head_commit("op_retry_idempotent", &"e".repeat(40))
            .unwrap();
        manager
            .transition("op_retry_idempotent", WorkspacePhase::Analyzing)
            .unwrap();

        manager
            .mark_retryable_failure("op_retry_idempotent", "interrupted")
            .unwrap();
        manager
            .mark_retryable_failure("op_retry_idempotent", "interrupted")
            .unwrap();

        let marker = manager.load_marker("op_retry_idempotent").unwrap();
        assert_eq!(marker.phase, WorkspacePhase::RetryableFailure);
        assert_eq!(marker.retry_attempts, 0);
        assert!(
            manager
                .paths()
                .workspace_dir("op_retry_idempotent")
                .unwrap()
                .is_dir()
        );
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_refuses_a_symlink_checkout_and_preserves_its_target() {
        let (home, manager, identity, scope) = manager();
        manager
            .create("op_symlink", &identity, &scope, None)
            .unwrap();
        let outside = home.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep.txt"), b"keep").unwrap();
        manager
            .transition("op_symlink", WorkspacePhase::Cloning)
            .unwrap();
        manager
            .record_head_commit("op_symlink", &"c".repeat(40))
            .unwrap();
        manager
            .transition("op_symlink", WorkspacePhase::Analyzing)
            .unwrap();
        manager
            .transition("op_symlink", WorkspacePhase::Persisting)
            .unwrap();
        manager
            .transition("op_symlink", WorkspacePhase::Verifying)
            .unwrap();
        manager
            .transition("op_symlink", WorkspacePhase::Verified)
            .unwrap();
        symlink(
            &outside,
            manager
                .paths()
                .workspace_repository_dir("op_symlink")
                .unwrap(),
        )
        .unwrap();

        assert!(matches!(
            manager.cleanup_after_verification("op_symlink"),
            Err(WorkspaceError::UnsafeWorkspacePath(_))
        ));
        assert!(outside.join("keep.txt").is_file());
        assert!(
            manager
                .paths()
                .workspace_dir("op_symlink")
                .unwrap()
                .is_dir()
        );
    }

    #[cfg(unix)]
    #[test]
    fn startup_scan_rejects_a_symlinked_codemind_root() {
        let home = TempDir::new().unwrap();
        let outside = home.path().join("outside-codemind");
        fs::create_dir(&outside).unwrap();
        fs::create_dir(outside.join("workspaces")).unwrap();
        symlink(&outside, home.path().join(".codemind")).unwrap();

        let paths = CodeMindPaths::from_home_dir(home.path()).unwrap();
        let manager = WorkspaceManager::new(paths);
        assert!(matches!(
            manager.scan_owned_workspaces(),
            Err(WorkspaceError::Paths(
                CodeMindPathsError::SymlinkDirectory { .. }
            ))
        ));
        assert!(outside.join("workspaces").is_dir());
    }

    #[test]
    fn scan_is_non_destructive_and_returns_only_valid_owned_markers() {
        let (_home, manager, identity, scope) = manager();
        manager.create("op_valid", &identity, &scope, None).unwrap();
        manager
            .create("op_corrupt", &identity, &scope, None)
            .unwrap();
        fs::write(
            manager.paths().workspace_marker_path("op_corrupt").unwrap(),
            b"{}",
        )
        .unwrap();

        let found = manager.scan_owned_workspaces().unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].operation_id, "op_valid");
        assert!(
            manager
                .paths()
                .workspace_dir("op_corrupt")
                .unwrap()
                .exists()
        );
    }

    #[test]
    fn local_directory_identity_cannot_be_used_to_create_owned_clone_workspace() {
        let (home, manager, _identity, scope) = manager();
        let local = home.path().join("local-repo");
        fs::create_dir(&local).unwrap();
        let local_identity = RepositoryIdentity::from_local_directory(&local).unwrap();
        assert!(matches!(
            manager.create("op_local", &local_identity, &scope, None),
            Err(WorkspaceError::RemoteIdentityRequired)
        ));
        assert!(!manager.paths().workspace_dir("op_local").unwrap().exists());
    }
}

#[cfg(test)]
mod metadata_contract_level2_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn local_path_metadata_keeps_only_the_opaque_identity_and_scoped_state_round_trips() {
        let home = TempDir::new().unwrap();
        let local_repository = home.path().join("private-work/CodeMind");
        fs::create_dir_all(&local_repository).unwrap();
        let identity = RepositoryIdentity::from_local_directory(&local_repository).unwrap();
        let paths = CodeMindPaths::from_home_dir(home.path()).unwrap();

        let metadata = RepositoryMetadata::new(identity.clone()).unwrap();
        metadata.save(&paths).unwrap();
        let serialized = fs::read_to_string(
            paths
                .repository_metadata_path(identity.repository_id())
                .unwrap(),
        )
        .unwrap();
        let local_path = local_repository.to_string_lossy();
        assert!(!serialized.contains(local_path.as_ref()));
        assert_eq!(
            RepositoryMetadata::load(&paths, identity.repository_id()).unwrap(),
            metadata
        );

        let workspace_id = "tenant:project:123";
        let scope_key = workspace_scope_key(workspace_id).unwrap();
        let state = RepositoryState::new_for_workspace(
            identity.repository_id().to_owned(),
            workspace_id,
            scope_key.clone(),
        )
        .unwrap();
        state.save(&paths).unwrap();
        let loaded = RepositoryState::load(&paths, identity.repository_id(), &scope_key).unwrap();
        assert_eq!(loaded, state);
        assert_eq!(loaded.workspace_id, workspace_id);
        assert_ne!(loaded.workspace_id, loaded.workspace_scope_key);
    }

    #[test]
    fn previous_repository_state_schema_is_migrated_when_loaded() {
        let home = TempDir::new().unwrap();
        let paths = CodeMindPaths::from_home_dir(home.path()).unwrap();
        let repository_id = "repo_schema_upgrade";
        let scope_key = workspace_scope_key("workspace-schema-upgrade").unwrap();
        let mut previous = RepositoryState::new_for_workspace(
            repository_id,
            "workspace-schema-upgrade",
            scope_key.clone(),
        )
        .unwrap();
        previous.schema_version = 6;
        paths
            .ensure_repository_scope_dir(repository_id, &scope_key)
            .unwrap();
        fs::write(
            paths
                .repository_state_path(repository_id, &scope_key)
                .unwrap(),
            serde_json::to_vec(&previous).unwrap(),
        )
        .unwrap();

        let loaded = RepositoryState::load(&paths, repository_id, &scope_key).unwrap();
        assert_eq!(loaded.schema_version, REPOSITORY_STATE_SCHEMA_VERSION);
        assert_eq!(loaded.workspace_id, "workspace-schema-upgrade");
        assert_eq!(loaded.cleanup_state, CleanupState::NotCreated);
    }

    #[test]
    fn previous_workspace_marker_schema_is_migrated_when_loaded() {
        let (_home, manager, identity, scope) = {
            let home = TempDir::new().unwrap();
            let paths = CodeMindPaths::from_home_dir(home.path()).unwrap();
            let manager = WorkspaceManager::new(paths);
            let identity =
                RepositoryIdentity::from_remote("https://github.com/owner/repo.git").unwrap();
            let scope = workspace_scope_key("workspace-marker-schema").unwrap();
            (home, manager, identity, scope)
        };
        manager
            .create("op_schema_upgrade", &identity, &scope, None)
            .unwrap();
        let marker_path = manager
            .paths()
            .workspace_marker_path("op_schema_upgrade")
            .unwrap();
        let mut serialized: serde_json::Value =
            serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
        serialized["schema_version"] = serde_json::json!(1);
        let _ = serialized.as_object_mut().unwrap().remove("retry_attempts");
        fs::write(&marker_path, serde_json::to_vec(&serialized).unwrap()).unwrap();

        let loaded = manager.load_marker("op_schema_upgrade").unwrap();
        assert_eq!(loaded.schema_version, WORKSPACE_MARKER_SCHEMA_VERSION);
        assert_eq!(loaded.retry_attempts, 0);
        assert_eq!(loaded.phase, WorkspacePhase::Preparing);
    }

    #[cfg(unix)]
    #[test]
    fn metadata_and_state_loaders_reject_symlinked_files() {
        use std::os::unix::fs::symlink;

        let home = TempDir::new().unwrap();
        let paths = CodeMindPaths::from_home_dir(home.path()).unwrap();
        let identity =
            RepositoryIdentity::from_remote("https://github.com/owner/repo.git").unwrap();
        let scope_key = workspace_scope_key("workspace-load-check").unwrap();
        let outside = home.path().join("outside.json");
        fs::write(&outside, b"{}").unwrap();

        paths
            .ensure_repository_dir(identity.repository_id())
            .unwrap();
        let repository_metadata = paths
            .repository_metadata_path(identity.repository_id())
            .unwrap();
        symlink(&outside, &repository_metadata).unwrap();
        assert!(matches!(
            RepositoryMetadata::load(&paths, identity.repository_id()),
            Err(RepositoryIdentityError::Paths(
                CodeMindPathsError::SymlinkFile { .. }
            ))
        ));

        fs::remove_file(&repository_metadata).unwrap();
        paths
            .ensure_repository_scope_dir(identity.repository_id(), &scope_key)
            .unwrap();
        let state_path = paths
            .repository_state_path(identity.repository_id(), &scope_key)
            .unwrap();
        symlink(&outside, &state_path).unwrap();
        assert!(matches!(
            RepositoryState::load(&paths, identity.repository_id(), &scope_key),
            Err(RepositoryStateError::Paths(
                CodeMindPathsError::SymlinkFile { .. }
            ))
        ));
    }
}
