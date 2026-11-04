//! Operation-owned temporary Git checkout workspaces.
//!
//! Every workspace is created beneath the canonical CodeMind `workspaces/`
//! directory and receives a marker before a clone is attempted. Cleanup only
//! removes a workspace when its marker, operation ID, repository identity, and
//! canonical parent relationships all match the path derived by CodeMind.

use crate::repository::{
    identity::{RepositoryIdentity, RepositoryIdentityError, RepositoryIdentityKind},
    paths::{CodeMindPaths, CodeMindPathsError},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(unix)]
use std::fs::DirBuilder;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;

pub const WORKSPACE_MARKER_SCHEMA_VERSION: u32 = 2;
const PREVIOUS_WORKSPACE_MARKER_SCHEMA_VERSION: u32 = 1;
const WORKSPACE_OWNER: &str = "codemind";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspacePhase {
    Preparing,
    Cloning,
    Analyzing,
    Persisting,
    Verifying,
    Verified,
    /// Analysis/persistence/verification failed before publication. The owned
    /// clone is retained so a later request can retry without recloning.
    RetryableFailure,
    CleanupPending,
    CleanupFailed,
}

/// The durable marker for one engine-owned temporary clone.
///
/// `workspace_root` and `repository_root` are recorded for diagnostics and
/// crash recovery, but cleanup independently derives and verifies both paths
/// from `CodeMindPaths` and `operation_id`; it never trusts these fields alone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceMarker {
    pub schema_version: u32,
    pub owner: String,
    pub owned_temporary_checkout: bool,
    pub operation_id: String,
    pub repository_id: String,
    pub workspace_scope_key: String,
    pub remote_locator: String,
    pub requested_ref: Option<String>,
    pub workspace_root: PathBuf,
    pub repository_root: PathBuf,
    pub phase: WorkspacePhase,
    pub cleanup_reason: Option<WorkspaceCleanupReason>,
    pub head_commit: Option<String>,
    /// A bounded machine-readable error code only. Raw process diagnostics may
    /// contain credentials and must not be stored in the marker.
    pub failure_code: Option<String>,
    /// Number of times an abandoned retryable workspace has been resumed.
    #[serde(default)]
    pub retry_attempts: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkspaceMarker {
    pub fn validate(&self) -> Result<(), WorkspaceError> {
        if self.schema_version != WORKSPACE_MARKER_SCHEMA_VERSION {
            return Err(WorkspaceError::UnsupportedMarkerVersion {
                actual: self.schema_version,
                supported: WORKSPACE_MARKER_SCHEMA_VERSION,
            });
        }
        if self.owner != WORKSPACE_OWNER || !self.owned_temporary_checkout {
            return Err(WorkspaceError::MarkerOwnershipMismatch);
        }
        validate_component("operation ID", &self.operation_id)?;
        validate_component("repository ID", &self.repository_id)?;
        validate_scope_key(&self.workspace_scope_key)?;
        let identity = RepositoryIdentity::from_remote(&self.remote_locator)?;
        if identity.remote_locator() != Some(self.remote_locator.as_str())
            || identity.repository_id() != self.repository_id.as_str()
        {
            return Err(WorkspaceError::MarkerIdentityMismatch);
        }
        if self.workspace_root.as_os_str().is_empty()
            || !self.workspace_root.is_absolute()
            || self.repository_root.as_os_str().is_empty()
            || !self.repository_root.is_absolute()
        {
            return Err(WorkspaceError::InvalidMarker);
        }
        if let Some(reference) = &self.requested_ref {
            validate_ref(reference)?;
        }
        if matches!(
            self.phase,
            WorkspacePhase::CleanupPending | WorkspacePhase::CleanupFailed
        ) && self.cleanup_reason.is_none()
        {
            return Err(WorkspaceError::InvalidMarker);
        }
        if self.phase == WorkspacePhase::Verified && self.head_commit.is_none() {
            return Err(WorkspaceError::VerifiedWorkspaceHasNoHead);
        }
        if self.phase == WorkspacePhase::RetryableFailure
            && (self.failure_code.is_none() || self.cleanup_reason.is_some())
        {
            return Err(WorkspaceError::InvalidMarker);
        }
        if self.cleanup_reason == Some(WorkspaceCleanupReason::GraphsVerified)
            && self.head_commit.is_none()
        {
            return Err(WorkspaceError::VerifiedWorkspaceHasNoHead);
        }
        if let Some(commit) = &self.head_commit {
            validate_commit_id(commit)?;
        }
        if let Some(code) = &self.failure_code {
            validate_component("failure code", code)?;
        }
        if self.updated_at < self.created_at {
            return Err(WorkspaceError::InvalidMarker);
        }
        Ok(())
    }
}

/// Owns temporary workspaces for remote Git repositories. It does not execute
/// Git commands; that concern belongs to the separate `git` module.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceCleanupReason {
    /// Graph persistence and read-after-write verification completed.
    GraphsVerified,
    /// Legacy cleanup reason retained for reading older markers. New failed
    /// operations are retained as `RetryableFailure` and are not auto-deleted.
    OperationFailed,
    /// A retryable operation was explicitly abandoned by an authorized caller.
    RetryAbandoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRetryMode {
    CloneAgain,
    ResumeExistingClone,
}

#[derive(Debug, Clone)]
pub struct WorkspaceManager {
    paths: CodeMindPaths,
}

impl WorkspaceManager {
    pub fn new(paths: CodeMindPaths) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &CodeMindPaths {
        &self.paths
    }

    /// Create a unique operation directory and atomically persist its marker
    /// before returning the destination to the future Git clone layer.
    pub(crate) fn create(
        &self,
        operation_id: &str,
        identity: &RepositoryIdentity,
        workspace_scope_key: &str,
        requested_ref: Option<&str>,
    ) -> Result<WorkspaceMarker, WorkspaceError> {
        validate_component("operation ID", operation_id)?;
        validate_component("repository ID", identity.repository_id())?;
        validate_scope_key(workspace_scope_key)?;
        if identity.kind() != RepositoryIdentityKind::Remote {
            return Err(WorkspaceError::RemoteIdentityRequired);
        }
        if let Some(reference) = requested_ref {
            validate_ref(reference)?;
        }
        // Round-trip the transport locator through the parser so a mutated or
        // deserialized identity cannot smuggle a credential-bearing URL into
        // the marker. Preserve an optional `.git` suffix for servers that need it.
        let supplied_remote = identity
            .remote_locator()
            .ok_or(WorkspaceError::RemoteIdentityRequired)?;
        let normalized_identity = RepositoryIdentity::from_remote(supplied_remote)?;
        if normalized_identity.repository_id() != identity.repository_id() {
            return Err(WorkspaceError::MarkerIdentityMismatch);
        }
        let remote_locator = normalized_identity
            .remote_locator()
            .ok_or(WorkspaceError::RemoteIdentityRequired)?
            .to_owned();

        self.paths.ensure_base_dirs()?;
        let workspace_root = self.paths.workspace_dir(operation_id)?;
        let repository_root = self.paths.workspace_repository_dir(operation_id)?;
        create_private_directory_exclusive(&workspace_root)?;

        let marker = WorkspaceMarker {
            schema_version: WORKSPACE_MARKER_SCHEMA_VERSION,
            owner: WORKSPACE_OWNER.to_owned(),
            owned_temporary_checkout: true,
            operation_id: operation_id.to_owned(),
            repository_id: identity.repository_id().to_owned(),
            workspace_scope_key: workspace_scope_key.to_owned(),
            remote_locator,
            requested_ref: requested_ref.map(str::to_owned),
            workspace_root: workspace_root.clone(),
            repository_root,
            phase: WorkspacePhase::Preparing,
            cleanup_reason: None,
            head_commit: None,
            failure_code: None,
            retry_attempts: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        if let Err(error) = self.write_marker(&marker) {
            // This directory was created exclusively by this call and cannot
            // contain an untrusted pre-existing workspace. Remove only this path.
            let _ = fs::remove_dir(&workspace_root);
            return Err(error);
        }
        Ok(marker)
    }

    pub fn load_marker(&self, operation_id: &str) -> Result<WorkspaceMarker, WorkspaceError> {
        validate_component("operation ID", operation_id)?;
        let workspace_root = self.paths.workspace_dir(operation_id)?;
        ensure_real_directory(&workspace_root)?;
        let marker_path = self.paths.workspace_marker_path(operation_id)?;
        ensure_regular_file_not_symlink(&marker_path)?;
        let mut marker: WorkspaceMarker = read_json(&marker_path)?;
        let has_phase26_retry_phase = marker.phase == WorkspacePhase::RetryableFailure;
        let has_phase26_abandonment_reason =
            marker.cleanup_reason == Some(WorkspaceCleanupReason::RetryAbandoned);
        let retry_attempts = marker.retry_attempts;
        migrate_workspace_marker_schema(
            &mut marker.schema_version,
            has_phase26_retry_phase,
            has_phase26_abandonment_reason,
            retry_attempts,
        )?;
        marker.validate()?;
        self.validate_marker_paths(&marker, operation_id)?;
        Ok(marker)
    }

    /// Advance the marker through the allowed lifecycle transitions.
    pub(crate) fn transition(
        &self,
        operation_id: &str,
        next: WorkspacePhase,
    ) -> Result<WorkspaceMarker, WorkspaceError> {
        let mut marker = self.load_marker(operation_id)?;
        if !allowed_transition(marker.phase, next) {
            return Err(WorkspaceError::InvalidTransition {
                from: marker.phase,
                to: next,
            });
        }
        marker.phase = next;
        marker.updated_at = Utc::now();
        marker.failure_code = None;
        self.write_marker(&marker)?;
        Ok(marker)
    }

    /// Record the checked-out commit once Git has resolved HEAD.
    pub(crate) fn record_head_commit(
        &self,
        operation_id: &str,
        commit: &str,
    ) -> Result<WorkspaceMarker, WorkspaceError> {
        validate_commit_id(commit)?;
        let mut marker = self.load_marker(operation_id)?;
        if !matches!(
            marker.phase,
            WorkspacePhase::Cloning | WorkspacePhase::Analyzing
        ) {
            return Err(WorkspaceError::HeadRecordedInInvalidPhase(marker.phase));
        }
        marker.head_commit = Some(commit.to_ascii_lowercase());
        marker.updated_at = Utc::now();
        self.write_marker(&marker)?;
        Ok(marker)
    }

    /// Store a safe machine-readable failure code. Do not store raw stderr,
    /// unredacted URLs, tokens, or arbitrary user text in the marker.
    pub(crate) fn record_failure_code(
        &self,
        operation_id: &str,
        failure_code: &str,
    ) -> Result<WorkspaceMarker, WorkspaceError> {
        validate_component("failure code", failure_code)?;
        let mut marker = self.load_marker(operation_id)?;
        marker.failure_code = Some(failure_code.to_owned());
        marker.updated_at = Utc::now();
        self.write_marker(&marker)?;
        Ok(marker)
    }

    /// Delete a workspace after the caller has completed graph persistence and
    /// read-after-write verification. The marker must already be in `Verified`
    /// state; retries after a failed removal retain the original cleanup reason.
    pub(crate) fn cleanup_after_verification(
        &self,
        operation_id: &str,
    ) -> Result<(), WorkspaceError> {
        let marker = self.load_marker(operation_id)?;
        if marker.phase == WorkspacePhase::Verified {
            return self.remove_workspace(marker, WorkspaceCleanupReason::GraphsVerified);
        }
        if matches!(
            marker.phase,
            WorkspacePhase::CleanupPending | WorkspacePhase::CleanupFailed
        ) && marker.cleanup_reason == Some(WorkspaceCleanupReason::GraphsVerified)
        {
            return self.remove_workspace(marker, WorkspaceCleanupReason::GraphsVerified);
        }
        Err(WorkspaceError::VerificationRequiredForCleanup)
    }

    /// Record a failed pre-publication attempt while retaining the owned clone
    /// for a safe retry. It never deletes workspace contents.
    pub(crate) fn mark_retryable_failure(
        &self,
        operation_id: &str,
        failure_code: &str,
    ) -> Result<WorkspaceMarker, WorkspaceError> {
        validate_component("failure code", failure_code)?;
        let mut marker = self.load_marker(operation_id)?;
        match marker.phase {
            WorkspacePhase::Preparing
            | WorkspacePhase::Cloning
            | WorkspacePhase::Analyzing
            | WorkspacePhase::Persisting
            | WorkspacePhase::Verifying
            | WorkspacePhase::RetryableFailure => {}
            WorkspacePhase::CleanupPending | WorkspacePhase::CleanupFailed
                if marker.cleanup_reason == Some(WorkspaceCleanupReason::OperationFailed) => {}
            phase => return Err(WorkspaceError::RetryNotAllowed(phase)),
        }
        marker.phase = WorkspacePhase::RetryableFailure;
        marker.cleanup_reason = None;
        marker.failure_code = Some(failure_code.to_owned());
        marker.updated_at = Utc::now();
        self.write_marker(&marker)?;
        Ok(marker)
    }

    /// Begin a retry. A complete clone (recorded HEAD) is kept intact and will
    /// be reopened. An incomplete clone is removed only from its validated,
    /// operation-owned repository destination before a fresh clone is attempted.
    pub(crate) fn begin_retry(
        &self,
        operation_id: &str,
    ) -> Result<WorkspaceRetryMode, WorkspaceError> {
        let mut marker = self.load_marker(operation_id)?;
        if marker.phase != WorkspacePhase::RetryableFailure {
            return Err(WorkspaceError::RetryNotAllowed(marker.phase));
        }
        marker.retry_attempts = marker.retry_attempts.saturating_add(1);
        marker.updated_at = Utc::now();

        if marker.head_commit.is_some() {
            // Resume only when the complete operation-owned checkout and its
            // Git metadata still exist as real directories. If the checkout is
            // simply absent, recreate it from the same marker. Symlinks and
            // unexpected .git file types are rejected instead of removed.
            match fs::symlink_metadata(&marker.repository_root) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                    return Err(WorkspaceError::UnsafeWorkspacePath(
                        marker.repository_root.clone(),
                    ));
                }
                Ok(_) => {
                    let git_dir = marker.repository_root.join(".git");
                    match fs::symlink_metadata(&git_dir) {
                        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                            return Err(WorkspaceError::UnsafeWorkspacePath(git_dir));
                        }
                        Ok(_) => {
                            // A retryable marker must retain a failure code
                            // until it advances to Analyzing.
                            self.write_marker(&marker)?;
                            return Ok(WorkspaceRetryMode::ResumeExistingClone);
                        }
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                        Err(source) => {
                            return Err(WorkspaceError::Io {
                                path: git_dir,
                                source,
                            });
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(WorkspaceError::Io {
                        path: marker.repository_root.clone(),
                        source,
                    });
                }
            }
        }

        self.remove_partial_checkout(&marker)?;
        marker.head_commit = None;
        marker.phase = WorkspacePhase::Preparing;
        marker.cleanup_reason = None;
        marker.failure_code = None;
        self.write_marker(&marker)?;
        Ok(WorkspaceRetryMode::CloneAgain)
    }

    /// Complete a retry by reopening the previously validated clone.
    pub(crate) fn resume_analysis(
        &self,
        operation_id: &str,
    ) -> Result<WorkspaceMarker, WorkspaceError> {
        let mut marker = self.load_marker(operation_id)?;
        if marker.phase != WorkspacePhase::RetryableFailure || marker.head_commit.is_none() {
            return Err(WorkspaceError::RetryNotAllowed(marker.phase));
        }
        marker.phase = WorkspacePhase::Analyzing;
        marker.failure_code = None;
        marker.updated_at = Utc::now();
        self.write_marker(&marker)?;
        Ok(marker)
    }

    /// Reconcile a marker after startup has independently verified the durable
    /// repository state and storage generation. The marker's HEAD must match
    /// the exact commit in the verified receipt.
    pub(crate) fn mark_verified_after_recovery(
        &self,
        operation_id: &str,
        verified_commit: &str,
    ) -> Result<WorkspaceMarker, WorkspaceError> {
        validate_commit_id(verified_commit)?;
        let mut marker = self.load_marker(operation_id)?;
        if !marker
            .head_commit
            .as_deref()
            .is_some_and(|head| head.eq_ignore_ascii_case(verified_commit))
        {
            return Err(WorkspaceError::RecoveryCommitMismatch);
        }
        if matches!(
            marker.phase,
            WorkspacePhase::CleanupPending | WorkspacePhase::CleanupFailed
        ) && marker.cleanup_reason == Some(WorkspaceCleanupReason::GraphsVerified)
        {
            return Ok(marker);
        }
        if matches!(
            marker.phase,
            WorkspacePhase::Preparing
                | WorkspacePhase::Cloning
                | WorkspacePhase::Analyzing
                | WorkspacePhase::Persisting
                | WorkspacePhase::Verifying
                | WorkspacePhase::RetryableFailure
                | WorkspacePhase::Verified
        ) {
            marker.phase = WorkspacePhase::Verified;
            marker.cleanup_reason = None;
            marker.failure_code = None;
            marker.updated_at = Utc::now();
            self.write_marker(&marker)?;
            return Ok(marker);
        }
        Err(WorkspaceError::RecoveryPhaseMismatch(marker.phase))
    }

    /// Delete a retryable workspace only after an explicit abandonment decision.
    /// Startup recovery never calls this automatically.
    pub(crate) fn abandon_retryable_workspace(
        &self,
        operation_id: &str,
    ) -> Result<(), WorkspaceError> {
        let mut marker = self.load_marker(operation_id)?;
        if marker.phase != WorkspacePhase::RetryableFailure {
            return Err(WorkspaceError::RetryNotAllowed(marker.phase));
        }
        marker.cleanup_reason = Some(WorkspaceCleanupReason::RetryAbandoned);
        self.remove_workspace(marker, WorkspaceCleanupReason::RetryAbandoned)
    }

    /// Retry a cleanup that was already explicitly authorized and durably
    /// recorded as abandoned. Startup may call this only after revalidating the
    /// original repository-scope lock and state ownership.
    pub(crate) fn retry_abandoned_cleanup(&self, operation_id: &str) -> Result<(), WorkspaceError> {
        let marker = self.load_marker(operation_id)?;
        if !matches!(
            marker.phase,
            WorkspacePhase::CleanupPending | WorkspacePhase::CleanupFailed
        ) || marker.cleanup_reason != Some(WorkspaceCleanupReason::RetryAbandoned)
        {
            return Err(WorkspaceError::CleanupReasonMismatch);
        }
        self.remove_workspace(marker, WorkspaceCleanupReason::RetryAbandoned)
    }

    fn remove_partial_checkout(&self, marker: &WorkspaceMarker) -> Result<(), WorkspaceError> {
        self.validate_marker_paths(marker, &marker.operation_id)?;
        let checkout = &marker.repository_root;
        match fs::symlink_metadata(checkout) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(source) => {
                return Err(WorkspaceError::Io {
                    path: checkout.clone(),
                    source,
                });
            }
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(WorkspaceError::UnsafeWorkspacePath(checkout.clone()));
            }
            Ok(_) => {}
        }
        let workspace =
            fs::canonicalize(&marker.workspace_root).map_err(|source| WorkspaceError::Io {
                path: marker.workspace_root.clone(),
                source,
            })?;
        let canonical_checkout =
            fs::canonicalize(checkout).map_err(|source| WorkspaceError::Io {
                path: checkout.clone(),
                source,
            })?;
        if canonical_checkout.parent() != Some(workspace.as_path()) {
            return Err(WorkspaceError::UnsafeWorkspacePath(checkout.clone()));
        }
        fs::remove_dir_all(checkout).map_err(|source| WorkspaceError::CleanupIo {
            path: checkout.clone(),
            source,
        })
    }

    fn remove_workspace(
        &self,
        mut marker: WorkspaceMarker,
        reason: WorkspaceCleanupReason,
    ) -> Result<(), WorkspaceError> {
        let operation_id = marker.operation_id.clone();
        self.validate_removal_target(&marker, &operation_id)?;
        marker.cleanup_reason = Some(reason);
        marker.phase = WorkspacePhase::CleanupPending;
        marker.updated_at = Utc::now();
        self.write_marker(&marker)?;

        let workspace_root = self.paths.workspace_dir(&operation_id)?;

        // Remove the large checkout before removing its ownership marker. If a
        // permission or filesystem error interrupts recursive checkout deletion,
        // the marker remains available for a later recovery attempt.
        if let Err(error) = self.remove_partial_checkout(&marker) {
            self.record_cleanup_failure(&marker);
            return Err(error);
        }

        // A workspace created by CodeMind has only the marker, the checkout
        // directory, and possible atomic-marker-write temporary files at its
        // top level. Refuse to erase unexpected entries rather than treating
        // every file that happens to appear there as operation-owned data.
        if let Err(error) = self.remove_marker_write_temporaries_and_validate_root(&marker) {
            self.record_cleanup_failure(&marker);
            return Err(error);
        }

        let marker_path = self.paths.workspace_marker_path(&operation_id)?;
        if let Err(source) = fs::remove_file(&marker_path) {
            self.record_cleanup_failure(&marker);
            return Err(WorkspaceError::CleanupIo {
                path: marker_path,
                source,
            });
        }

        match fs::remove_dir(&workspace_root) {
            Ok(()) => Ok(()),
            Err(source) => {
                // The marker is removed only after the checkout and all
                // expected temporary entries are gone. If removing the now
                // empty workspace directory fails, restore a cleanup-failed
                // marker whenever the original directory is still safe.
                let mut failed = marker;
                failed.phase = WorkspacePhase::CleanupFailed;
                failed.failure_code = Some("cleanup_failed".to_owned());
                failed.updated_at = Utc::now();
                let _ = self.write_marker(&failed);
                Err(WorkspaceError::CleanupIo {
                    path: workspace_root,
                    source,
                })
            }
        }
    }

    fn remove_marker_write_temporaries_and_validate_root(
        &self,
        marker: &WorkspaceMarker,
    ) -> Result<(), WorkspaceError> {
        self.validate_marker_paths(marker, &marker.operation_id)?;
        let marker_path = self.paths.workspace_marker_path(&marker.operation_id)?;
        ensure_regular_file_not_symlink(&marker_path)?;
        let entries =
            fs::read_dir(&marker.workspace_root).map_err(|source| WorkspaceError::Io {
                path: marker.workspace_root.clone(),
                source,
            })?;
        for entry in entries {
            let entry = entry.map_err(|source| WorkspaceError::Io {
                path: marker.workspace_root.clone(),
                source,
            })?;
            let path = entry.path();
            if path == marker_path {
                continue;
            }
            let file_type = entry.file_type().map_err(|source| WorkspaceError::Io {
                path: path.clone(),
                source,
            })?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let is_atomic_marker_temporary = name.starts_with(".workspace-")
                && name.ends_with(".tmp")
                && file_type.is_file()
                && !file_type.is_symlink();
            if !is_atomic_marker_temporary {
                return Err(WorkspaceError::UnexpectedWorkspaceEntry(path));
            }
            fs::remove_file(&path).map_err(|source| WorkspaceError::CleanupIo {
                path: path.clone(),
                source,
            })?;
        }
        Ok(())
    }

    fn record_cleanup_failure(&self, marker: &WorkspaceMarker) {
        if let Ok(mut current) = self.load_marker(&marker.operation_id) {
            current.phase = WorkspacePhase::CleanupFailed;
            current.updated_at = Utc::now();
            current.failure_code = Some("cleanup_failed".to_owned());
            let _ = self.write_marker(&current);
        }
    }

    /// Non-destructively list valid engine-owned workspaces for startup recovery.
    /// Invalid/missing markers and symlinked entries are skipped, never cleaned.
    /// The caller must determine whether a valid candidate is truly abandoned
    /// before invoking `cleanup`, because a scan alone cannot establish that no
    /// concurrent operation is still using it.
    pub fn scan_owned_workspaces(&self) -> Result<Vec<WorkspaceMarker>, WorkspaceError> {
        let Some(root) = self.paths.existing_workspaces_dir()? else {
            return Ok(Vec::new());
        };

        let entries = fs::read_dir(&root).map_err(|source| WorkspaceError::Io {
            path: root.clone(),
            source,
        })?;
        let mut workspaces = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| WorkspaceError::Io {
                path: root.clone(),
                source,
            })?;
            let file_type = entry.file_type().map_err(|source| WorkspaceError::Io {
                path: entry.path(),
                source,
            })?;
            if file_type.is_symlink() || !file_type.is_dir() {
                continue;
            }
            let Some(operation_id) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if let Ok(marker) = self.load_marker(&operation_id) {
                workspaces.push(marker);
            }
        }
        workspaces.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
        Ok(workspaces)
    }

    fn write_marker(&self, marker: &WorkspaceMarker) -> Result<(), WorkspaceError> {
        marker.validate()?;
        self.validate_marker_paths(marker, &marker.operation_id)?;
        let path = self.paths.workspace_marker_path(&marker.operation_id)?;
        write_json_atomically(&path, marker)
    }

    fn validate_marker_paths(
        &self,
        marker: &WorkspaceMarker,
        requested_operation_id: &str,
    ) -> Result<(), WorkspaceError> {
        if marker.operation_id != requested_operation_id {
            return Err(WorkspaceError::MarkerIdentityMismatch);
        }
        let expected_workspace_root = self.paths.workspace_dir(requested_operation_id)?;
        let expected_repository_root = self
            .paths
            .workspace_repository_dir(requested_operation_id)?;
        if marker.workspace_root != expected_workspace_root
            || marker.repository_root != expected_repository_root
        {
            return Err(WorkspaceError::MarkerPathMismatch);
        }

        let workspaces_root = self
            .paths
            .existing_workspaces_dir()?
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound(requested_operation_id.to_owned()))?;
        let canonical_workspaces =
            fs::canonicalize(&workspaces_root).map_err(|source| WorkspaceError::Io {
                path: workspaces_root.clone(),
                source,
            })?;
        let canonical_workspace =
            fs::canonicalize(&expected_workspace_root).map_err(|source| WorkspaceError::Io {
                path: expected_workspace_root.clone(),
                source,
            })?;
        if canonical_workspace.parent() != Some(canonical_workspaces.as_path()) {
            return Err(WorkspaceError::UnsafeWorkspacePath(expected_workspace_root));
        }

        let repository_root = &expected_repository_root;
        match fs::symlink_metadata(repository_root) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(WorkspaceError::UnsafeWorkspacePath(repository_root.clone()));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(WorkspaceError::Io {
                    path: repository_root.clone(),
                    source,
                });
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(WorkspaceError::UnsafeWorkspacePath(repository_root.clone()));
            }
            Ok(_) => {
                let canonical_repository =
                    fs::canonicalize(repository_root).map_err(|source| WorkspaceError::Io {
                        path: repository_root.clone(),
                        source,
                    })?;
                if canonical_repository.parent() != Some(canonical_workspace.as_path()) {
                    return Err(WorkspaceError::UnsafeWorkspacePath(repository_root.clone()));
                }
            }
        }
        Ok(())
    }

    fn validate_removal_target(
        &self,
        marker: &WorkspaceMarker,
        operation_id: &str,
    ) -> Result<(), WorkspaceError> {
        self.validate_marker_paths(marker, operation_id)?;
        let target = self.paths.workspace_dir(operation_id)?;
        let metadata = fs::symlink_metadata(&target).map_err(|source| WorkspaceError::Io {
            path: target.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(WorkspaceError::UnsafeWorkspacePath(target));
        }
        let marker_path = self.paths.workspace_marker_path(operation_id)?;
        ensure_regular_file_not_symlink(&marker_path)?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("a remote repository identity is required for a temporary clone workspace")]
    RemoteIdentityRequired,
    #[error("invalid {kind} component for workspace management")]
    InvalidComponent { kind: &'static str },
    #[error("invalid workspace-scope key")]
    InvalidWorkspaceScopeKey,
    #[error("workspace operation `{0}` already exists; refusing to take ownership")]
    WorkspaceAlreadyExists(String),
    #[error("workspace operation `{0}` does not exist")]
    WorkspaceNotFound(String),
    #[error("workspace marker is missing or is not a regular file")]
    MarkerMissing,
    #[error("workspace marker is corrupt or inconsistent")]
    InvalidMarker,
    #[error("workspace marker schema version {actual} is not supported (supported: {supported})")]
    UnsupportedMarkerVersion { actual: u32, supported: u32 },
    #[error("workspace marker claims ownership that does not match CodeMind's ownership contract")]
    MarkerOwnershipMismatch,
    #[error("workspace marker identity does not match its repository locator or operation")]
    MarkerIdentityMismatch,
    #[error("workspace marker paths do not match paths derived from the CodeMind root")]
    MarkerPathMismatch,
    #[error(
        "refusing to operate on a symlink, escaped path, or non-directory workspace path: {0:?}"
    )]
    UnsafeWorkspacePath(PathBuf),
    #[error("invalid workspace transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: WorkspacePhase,
        to: WorkspacePhase,
    },
    #[error("graph verification must complete before successful-path workspace cleanup")]
    VerificationRequiredForCleanup,
    #[error("cleanup reason does not match the operation's recorded lifecycle")]
    CleanupReasonMismatch,
    #[error("retry is not allowed from workspace phase {0:?}")]
    RetryNotAllowed(WorkspacePhase),
    #[error("recovered repository state does not match the workspace HEAD")]
    RecoveryCommitMismatch,
    #[error("recovery cannot reconcile workspace phase {0:?}")]
    RecoveryPhaseMismatch(WorkspacePhase),
    #[error("a workspace cannot be marked verified without a resolved HEAD commit")]
    VerifiedWorkspaceHasNoHead,
    #[error("cannot record HEAD while workspace is in phase {0:?}")]
    HeadRecordedInInvalidPhase(WorkspacePhase),
    #[error("invalid commit ID; expected 40 or 64 hexadecimal characters")]
    InvalidCommitId,
    #[error("invalid requested Git ref")]
    InvalidRef,
    #[error("workspace metadata filesystem operation failed at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("workspace cleanup failed at {path:?}: {source}")]
    CleanupIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("refusing to remove unexpected content from an owned workspace: {0:?}")]
    UnexpectedWorkspaceEntry(PathBuf),
    #[error("workspace marker serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Paths(#[from] CodeMindPathsError),
    #[error(transparent)]
    Identity(#[from] RepositoryIdentityError),
}

/// Upgrade the previous marker format in memory. Version 2 adds retry
/// bookkeeping and new lifecycle enum variants while retaining all version-1
/// fields; the first subsequent marker write persists the upgraded version.
fn migrate_workspace_marker_schema(
    schema_version: &mut u32,
    has_phase26_retry_phase: bool,
    has_phase26_abandonment_reason: bool,
    retry_attempts: u32,
) -> Result<(), WorkspaceError> {
    match *schema_version {
        PREVIOUS_WORKSPACE_MARKER_SCHEMA_VERSION => {
            // These values did not exist in marker schema v1. Do not let a
            // record claiming the old version smuggle in Phase 26 retry or
            // abandonment authority during migration.
            if has_phase26_retry_phase || has_phase26_abandonment_reason || retry_attempts != 0 {
                return Err(WorkspaceError::InvalidMarker);
            }
            *schema_version = WORKSPACE_MARKER_SCHEMA_VERSION;
            Ok(())
        }
        WORKSPACE_MARKER_SCHEMA_VERSION => Ok(()),
        actual => Err(WorkspaceError::UnsupportedMarkerVersion {
            actual,
            supported: WORKSPACE_MARKER_SCHEMA_VERSION,
        }),
    }
}

fn allowed_transition(from: WorkspacePhase, to: WorkspacePhase) -> bool {
    matches!(
        (from, to),
        (WorkspacePhase::Preparing, WorkspacePhase::Cloning)
            | (WorkspacePhase::Cloning, WorkspacePhase::Analyzing)
            | (WorkspacePhase::Analyzing, WorkspacePhase::Persisting)
            | (WorkspacePhase::Persisting, WorkspacePhase::Verifying)
            | (WorkspacePhase::Verifying, WorkspacePhase::Verified)
            | (WorkspacePhase::Preparing, WorkspacePhase::RetryableFailure)
            | (WorkspacePhase::Cloning, WorkspacePhase::RetryableFailure)
            | (WorkspacePhase::Analyzing, WorkspacePhase::RetryableFailure)
            | (WorkspacePhase::Persisting, WorkspacePhase::RetryableFailure)
            | (WorkspacePhase::Verifying, WorkspacePhase::RetryableFailure)
            | (WorkspacePhase::RetryableFailure, WorkspacePhase::Preparing)
            | (WorkspacePhase::RetryableFailure, WorkspacePhase::Analyzing)
            | (
                WorkspacePhase::CleanupPending,
                WorkspacePhase::CleanupFailed
            )
            | (
                WorkspacePhase::CleanupFailed,
                WorkspacePhase::CleanupPending
            )
    )
}

fn validate_component(kind: &'static str, value: &str) -> Result<(), WorkspaceError> {
    if value.is_empty()
        || value.len() > 512
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(WorkspaceError::InvalidComponent { kind });
    }
    Ok(())
}

fn validate_scope_key(value: &str) -> Result<(), WorkspaceError> {
    let suffix = value.strip_prefix("scope-").unwrap_or_default();
    if suffix.len() != 64
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(WorkspaceError::InvalidWorkspaceScopeKey);
    }
    Ok(())
}

fn validate_commit_id(value: &str) -> Result<(), WorkspaceError> {
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(WorkspaceError::InvalidCommitId);
    }
    Ok(())
}

fn validate_ref(value: &str) -> Result<(), WorkspaceError> {
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
        return Err(WorkspaceError::InvalidRef);
    }
    Ok(())
}

fn create_private_directory_exclusive(path: &Path) -> Result<(), WorkspaceError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = DirBuilder::new();
        builder.mode(0o700);
        builder.create(path).map_err(|source| {
            if source.kind() == io::ErrorKind::AlreadyExists {
                WorkspaceError::WorkspaceAlreadyExists(
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown")
                        .to_owned(),
                )
            } else {
                WorkspaceError::Io {
                    path: path.to_path_buf(),
                    source,
                }
            }
        })?;
    }
    #[cfg(not(unix))]
    {
        fs::create_dir(path).map_err(|source| {
            if source.kind() == io::ErrorKind::AlreadyExists {
                WorkspaceError::WorkspaceAlreadyExists(
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown")
                        .to_owned(),
                )
            } else {
                WorkspaceError::Io {
                    path: path.to_path_buf(),
                    source,
                }
            }
        })?;
    }
    Ok(())
}

fn ensure_real_directory(path: &Path) -> Result<(), WorkspaceError> {
    match fs::symlink_metadata(path) {
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            let operation_id = path
                .file_name()
                .and_then(|part| part.to_str())
                .unwrap_or_default();
            Err(WorkspaceError::WorkspaceNotFound(operation_id.to_owned()))
        }
        Err(source) => Err(WorkspaceError::Io {
            path: path.to_path_buf(),
            source,
        }),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(WorkspaceError::UnsafeWorkspacePath(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
    }
}

fn ensure_regular_file_not_symlink(path: &Path) -> Result<(), WorkspaceError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(WorkspaceError::MarkerMissing),
        Err(source) => Err(WorkspaceError::Io {
            path: path.to_path_buf(),
            source,
        }),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(WorkspaceError::MarkerMissing)
        }
        Ok(_) => Ok(()),
    }
}

fn write_json_atomically<T: Serialize>(
    destination: &Path,
    value: &T,
) -> Result<(), WorkspaceError> {
    let parent = destination.parent().ok_or(WorkspaceError::InvalidMarker)?;
    let temporary = parent.join(format!(".workspace-{}.tmp", Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(value)?;
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|source| WorkspaceError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.write_all(&bytes)
            .map_err(|source| WorkspaceError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.sync_all().map_err(|source| WorkspaceError::Io {
            path: temporary.clone(),
            source,
        })?;
        fs::rename(&temporary, destination).map_err(|source| WorkspaceError::Io {
            path: destination.to_path_buf(),
            source,
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, WorkspaceError> {
    let bytes = fs::read(path).map_err(|source| WorkspaceError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod level1_tests {
    use super::{WORKSPACE_MARKER_SCHEMA_VERSION, WorkspaceError, migrate_workspace_marker_schema};

    #[test]
    fn previous_workspace_marker_schema_is_upgraded_and_future_or_mixed_versions_are_rejected() {
        let mut previous = 1;
        migrate_workspace_marker_schema(&mut previous, false, false, 0).unwrap();
        assert_eq!(previous, WORKSPACE_MARKER_SCHEMA_VERSION);

        let mut current = WORKSPACE_MARKER_SCHEMA_VERSION;
        migrate_workspace_marker_schema(&mut current, true, true, 2).unwrap();
        assert_eq!(current, WORKSPACE_MARKER_SCHEMA_VERSION);

        let mut mixed_previous = 1;
        assert!(matches!(
            migrate_workspace_marker_schema(&mut mixed_previous, false, true, 0),
            Err(WorkspaceError::InvalidMarker)
        ));
        assert_eq!(mixed_previous, 1);

        let mut future = WORKSPACE_MARKER_SCHEMA_VERSION + 1;
        assert!(matches!(
            migrate_workspace_marker_schema(&mut future, false, false, 0),
            Err(WorkspaceError::UnsupportedMarkerVersion { .. })
        ));
        assert_eq!(future, WORKSPACE_MARKER_SCHEMA_VERSION + 1);
    }
}
