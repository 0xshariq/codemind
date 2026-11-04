//! Durable repository analysis state.
//!
//! Repository readiness, knowledge completeness, freshness, and temporary
//! workspace cleanup are modeled independently. A JSON state record is only a
//! local index: graph reuse must also verify that the corresponding facts still
//! exist in the configured graph store.

use crate::repository::{
    identity::{RepositoryIdentityError, workspace_scope_key as derive_workspace_scope_key},
    paths::{CodeMindPaths, CodeMindPathsError},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};
use thiserror::Error;
use uuid::Uuid;

pub const REPOSITORY_STATE_SCHEMA_VERSION: u32 = 7;
const PREVIOUS_REPOSITORY_STATE_SCHEMA_VERSION: u32 = 6;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisState {
    Unknown,
    Preparing,
    Analyzing,
    Persisting,
    Verifying,
    Ready,
    Stale,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupState {
    NotCreated,
    Active,
    /// An unverified owned clone is intentionally retained for a later retry.
    RetryableFailure,
    CleanupPending,
    Cleaned,
    CleanupFailed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeCompleteness {
    Unknown,
    Complete,
    Partial,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RemoteFreshnessState {
    Unknown,
    Unchanged,
    Changed,
}

/// Proof metadata returned by a storage-layer read-after-write verification.
/// Counts are observations, not a replacement for running verification again
/// before a future graph reuse decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GraphVerificationReceipt {
    pub verification_id: String,
    /// Unique publication generation for this verified repository state.
    /// It is distinct from the Git commit, source snapshot, and graph projection version.
    pub generation_id: String,
    pub verified_at: DateTime<Utc>,
    pub graph_names: Vec<String>,
    pub repository_id: String,
    pub workspace_id: String,
    pub source_snapshot_id: String,
    pub graph_projection_version: String,
    pub verified_commit: String,
    pub code_entities: u64,
    pub code_relationships: u64,
    pub code_entity_ids_digest: String,
    pub code_relationship_ids_digest: String,
    pub test_entities: u64,
    pub test_relationships: u64,
    pub test_entity_ids_digest: String,
    pub test_relationship_ids_digest: String,
    pub history_entities: u64,
    pub history_relationships: u64,
    pub history_head_commit: String,
    pub history_complete: bool,
    pub history_omitted_commits_at_least: u64,
    pub history_omitted_path_changes_at_least: u64,
    pub change_entities: u64,
    pub change_relationships: u64,
}

impl GraphVerificationReceipt {
    pub fn validate(&self) -> Result<(), RepositoryStateError> {
        let unique_graph_names = {
            let mut names = self.graph_names.clone();
            names.sort();
            names.dedup();
            names.len() == self.graph_names.len()
        };
        if !is_safe_component(&self.verification_id)
            || self.graph_names.is_empty()
            || self.graph_names.len() > 16
            || !unique_graph_names
            || !["code", "history", "test"].iter().all(|required| {
                self.graph_names
                    .iter()
                    .any(|name| name.as_str() == *required)
            })
            || self.graph_names.iter().any(|name| !is_safe_component(name))
            || !is_safe_component(&self.repository_id)
            || !is_valid_workspace_id(&self.workspace_id)
            || !is_safe_component(&self.source_snapshot_id)
            || !is_safe_component(&self.graph_projection_version)
            || !is_safe_component(&self.generation_id)
            || !is_sha256_hex(&self.code_entity_ids_digest)
            || !is_sha256_hex(&self.code_relationship_ids_digest)
            || !is_sha256_hex(&self.test_entity_ids_digest)
            || !is_sha256_hex(&self.test_relationship_ids_digest)
            || validate_commit_id(&self.verified_commit).is_err()
            || validate_commit_id(&self.history_head_commit).is_err()
            || (self.history_complete
                && (self.history_omitted_commits_at_least != 0
                    || self.history_omitted_path_changes_at_least != 0))
        {
            return Err(RepositoryStateError::InvalidVerificationReceipt);
        }
        Ok(())
    }
}

/// Durable description of the graph knowledge currently owned by one
/// repository scope. This is intentionally separate from the broader
/// repository lifecycle state so callers can make a reuse/update decision
/// without reconstructing source or Git history first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryGraphState {
    pub repository_id: String,
    pub workspace_id: String,
    pub source_snapshot_id: String,
    pub graph_projection_version: String,
    pub generation_id: String,
    pub verified_commit: String,
    pub completeness: KnowledgeCompleteness,
    pub verification: GraphVerificationReceipt,
}

impl RepositoryGraphState {
    pub fn is_usable(&self) -> bool {
        self.completeness != KnowledgeCompleteness::Unknown
            && !self.verification.graph_names.is_empty()
            && self.verification.repository_id == self.repository_id
            && self.verification.workspace_id == self.workspace_id
            && self.verification.source_snapshot_id == self.source_snapshot_id
            && self.verification.graph_projection_version == self.graph_projection_version
            && self.verification.generation_id == self.generation_id
            && self.verification.verified_commit == self.verified_commit
    }

    pub fn matches_commit(&self, current_commit: &str) -> bool {
        self.verified_commit.eq_ignore_ascii_case(current_commit)
    }
}

/// Latest verified state for a repository in one workspace/security scope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepositoryState {
    pub schema_version: u32,
    pub repository_id: String,
    pub workspace_scope_key: String,
    #[serde(default = "default_workspace_id")]
    pub workspace_id: String,
    pub analysis_state: AnalysisState,
    pub cleanup_state: CleanupState,
    pub last_workspace_operation_id: Option<String>,
    pub normalized_ref: Option<String>,
    pub default_branch: Option<String>,
    pub last_verified_commit: Option<String>,
    pub last_source_snapshot_id: Option<String>,
    pub graph_projection_version: Option<String>,
    pub verification: Option<GraphVerificationReceipt>,
    pub completeness: KnowledgeCompleteness,
    pub omitted_file_count: u64,
    pub last_remote_head: Option<String>,
    pub last_remote_checked_at: Option<DateTime<Utc>>,
    pub last_remote_freshness: RemoteFreshnessState,
    /// Read-only fingerprint of the caller-owned Git HEAD/index/worktree status
    /// captured alongside the source snapshot. `None` deliberately disables
    /// local reuse because older state cannot prove this part of freshness.
    #[serde(default)]
    pub last_local_git_state_digest: Option<String>,
    pub last_successful_analysis_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RepositoryState {
    pub fn new(
        repository_id: impl Into<String>,
        workspace_scope_key: impl Into<String>,
    ) -> Result<Self, RepositoryStateError> {
        let workspace_scope_key = workspace_scope_key.into();
        // Preserve the legacy constructor contract: callers that provide only
        // a scope key have no separate workspace ID in their state record.
        Self::new_with_workspace_identity(
            repository_id,
            workspace_scope_key.clone(),
            workspace_scope_key,
        )
    }

    /// Construct durable state with the real workspace identity while keeping
    /// the opaque scope key separate for filesystem isolation.
    pub fn new_for_workspace(
        repository_id: impl Into<String>,
        workspace_id: impl Into<String>,
        workspace_scope_key: impl Into<String>,
    ) -> Result<Self, RepositoryStateError> {
        // Build the final identity tuple before validation. Calling `new()`
        // here would validate an intermediate state with workspace_id equal
        // to the opaque scope key, which is not a valid derived scope for the
        // caller's actual workspace ID.
        Self::new_with_workspace_identity(repository_id, workspace_id, workspace_scope_key)
    }

    fn new_with_workspace_identity(
        repository_id: impl Into<String>,
        workspace_id: impl Into<String>,
        workspace_scope_key: impl Into<String>,
    ) -> Result<Self, RepositoryStateError> {
        let now = Utc::now();
        let state = Self {
            schema_version: REPOSITORY_STATE_SCHEMA_VERSION,
            repository_id: repository_id.into(),
            workspace_scope_key: workspace_scope_key.into(),
            workspace_id: workspace_id.into(),
            analysis_state: AnalysisState::Unknown,
            cleanup_state: CleanupState::NotCreated,
            last_workspace_operation_id: None,
            normalized_ref: None,
            default_branch: None,
            last_verified_commit: None,
            last_source_snapshot_id: None,
            graph_projection_version: None,
            verification: None,
            completeness: KnowledgeCompleteness::Unknown,
            omitted_file_count: 0,
            last_remote_head: None,
            last_remote_checked_at: None,
            last_remote_freshness: RemoteFreshnessState::Unknown,
            last_local_git_state_digest: None,
            last_successful_analysis_at: None,
            created_at: now,
            updated_at: now,
        };
        state.validate()?;
        Ok(state)
    }

    /// Return the durable graph portion of a repository state when the state
    /// has reached `Ready`. Non-ready repositories deliberately return `None`;
    /// callers must not infer graph usability from a partially written state.
    pub fn graph_state(&self) -> Result<Option<RepositoryGraphState>, RepositoryStateError> {
        if self.analysis_state != AnalysisState::Ready {
            return Ok(None);
        }

        let graph_state = RepositoryGraphState {
            repository_id: self.repository_id.clone(),
            workspace_id: self.workspace_id.clone(),
            source_snapshot_id: self
                .last_source_snapshot_id
                .clone()
                .ok_or(RepositoryStateError::ReadyRequiresVerifiedGraph)?,
            graph_projection_version: self
                .graph_projection_version
                .clone()
                .ok_or(RepositoryStateError::ReadyRequiresVerifiedGraph)?,
            generation_id: self
                .verification
                .as_ref()
                .ok_or(RepositoryStateError::ReadyRequiresVerifiedGraph)?
                .generation_id
                .clone(),
            verified_commit: self
                .last_verified_commit
                .clone()
                .ok_or(RepositoryStateError::ReadyRequiresVerifiedGraph)?,
            completeness: self.completeness,
            verification: self
                .verification
                .clone()
                .ok_or(RepositoryStateError::ReadyRequiresVerifiedGraph)?,
        };
        Ok(Some(graph_state))
    }

    /// Advance the analysis lifecycle while preserving the independent cleanup
    /// lifecycle. `Ready` must be entered through `mark_ready` so the graph
    /// verification metadata cannot be skipped.
    pub fn transition_analysis(&mut self, next: AnalysisState) -> Result<(), RepositoryStateError> {
        let allowed = matches!(
            (self.analysis_state, next),
            (AnalysisState::Unknown, AnalysisState::Preparing)
                | (AnalysisState::Preparing, AnalysisState::Analyzing)
                | (AnalysisState::Preparing, AnalysisState::Failed)
                | (AnalysisState::Analyzing, AnalysisState::Persisting)
                | (AnalysisState::Analyzing, AnalysisState::Failed)
                | (AnalysisState::Persisting, AnalysisState::Verifying)
                | (AnalysisState::Persisting, AnalysisState::Failed)
                | (AnalysisState::Verifying, AnalysisState::Failed)
                | (AnalysisState::Ready, AnalysisState::Stale)
                | (AnalysisState::Stale, AnalysisState::Preparing)
                | (AnalysisState::Failed, AnalysisState::Preparing)
        );
        if !allowed {
            return Err(RepositoryStateError::InvalidTransition {
                from: self.analysis_state,
                to: next,
            });
        }
        self.analysis_state = next;
        self.updated_at = Utc::now();
        self.validate()
    }

    /// Commit the latest verified graph pointer. This is the only API that can
    /// promote a repository state to `Ready`.
    #[allow(clippy::too_many_arguments)]
    pub fn mark_ready(
        &mut self,
        commit: &str,
        source_snapshot_id: &str,
        graph_projection_version: &str,
        generation_id: &str,
        verification: GraphVerificationReceipt,
        completeness: KnowledgeCompleteness,
        omitted_file_count: u64,
        normalized_ref: Option<&str>,
    ) -> Result<(), RepositoryStateError> {
        if self.analysis_state != AnalysisState::Verifying {
            return Err(RepositoryStateError::ReadyRequiresVerificationState);
        }
        validate_commit_id(commit)?;
        validate_text_identity("source snapshot ID", source_snapshot_id)?;
        validate_text_identity("graph projection version", graph_projection_version)?;
        validate_text_identity("generation ID", generation_id)?;
        if let Some(reference) = normalized_ref {
            validate_ref(reference)?;
        }
        verification.validate()?;
        if verification.repository_id != self.repository_id
            || verification.workspace_id != self.workspace_id
            || verification.source_snapshot_id != source_snapshot_id
            || verification.graph_projection_version != graph_projection_version
            || verification.generation_id != generation_id
            || !verification.verified_commit.eq_ignore_ascii_case(commit)
            || !verification
                .history_head_commit
                .eq_ignore_ascii_case(commit)
        {
            return Err(RepositoryStateError::InvalidVerificationReceipt);
        }
        if completeness == KnowledgeCompleteness::Unknown {
            return Err(RepositoryStateError::UnknownCompletenessForReady);
        }
        if completeness == KnowledgeCompleteness::Complete
            && (omitted_file_count != 0
                || !verification.history_complete
                || verification.history_omitted_commits_at_least != 0
                || verification.history_omitted_path_changes_at_least != 0)
        {
            return Err(RepositoryStateError::CompleteStateHasOmissions);
        }

        self.last_verified_commit = Some(commit.to_ascii_lowercase());
        self.last_source_snapshot_id = Some(source_snapshot_id.to_owned());
        self.graph_projection_version = Some(graph_projection_version.to_owned());
        self.verification = Some(verification);
        self.completeness = completeness;
        self.omitted_file_count = omitted_file_count;
        self.normalized_ref = normalized_ref.map(str::to_owned);
        let now = Utc::now();
        self.last_successful_analysis_at = Some(now);
        self.analysis_state = AnalysisState::Ready;
        self.updated_at = now;
        self.validate()
    }

    pub fn record_remote_observation(
        &mut self,
        head: Option<&str>,
        freshness: RemoteFreshnessState,
        checked_at: DateTime<Utc>,
    ) -> Result<(), RepositoryStateError> {
        match (freshness, head) {
            (RemoteFreshnessState::Unknown, None) => {}
            (RemoteFreshnessState::Unknown, Some(_))
            | (RemoteFreshnessState::Unchanged | RemoteFreshnessState::Changed, None) => {
                return Err(RepositoryStateError::InconsistentRemoteObservation);
            }
            (RemoteFreshnessState::Unchanged | RemoteFreshnessState::Changed, Some(head)) => {
                validate_commit_id(head)?;
                self.last_remote_head = Some(head.to_ascii_lowercase());
            }
        }
        // An unknown observation must not erase the last successfully observed
        // HEAD. It does, however, replace the freshness claim with `Unknown`.
        self.last_remote_checked_at = Some(checked_at);
        self.last_remote_freshness = freshness;
        self.updated_at = Utc::now();
        self.validate()
    }

    /// Store the Git-state fingerprint associated with the source snapshot.
    /// `None` means the engine could not establish a complete local fingerprint
    /// and therefore must not reuse this state for a caller-owned checkout.
    pub fn record_local_git_state_digest(
        &mut self,
        digest: Option<&str>,
    ) -> Result<(), RepositoryStateError> {
        if let Some(digest) = digest {
            if !is_sha256_hex(digest) {
                return Err(RepositoryStateError::InvalidLocalGitStateDigest);
            }
            self.last_local_git_state_digest = Some(digest.to_ascii_lowercase());
        } else {
            self.last_local_git_state_digest = None;
        }
        self.updated_at = Utc::now();
        self.validate()
    }

    pub fn set_cleanup_state(
        &mut self,
        next: CleanupState,
        operation_id: Option<&str>,
    ) -> Result<(), RepositoryStateError> {
        let allowed = matches!(
            (self.cleanup_state, next),
            (CleanupState::NotCreated, CleanupState::Active)
                | (CleanupState::Active, CleanupState::RetryableFailure)
                | (CleanupState::RetryableFailure, CleanupState::Active)
                | (CleanupState::RetryableFailure, CleanupState::CleanupPending)
                | (CleanupState::CleanupPending, CleanupState::RetryableFailure)
                | (CleanupState::CleanupFailed, CleanupState::RetryableFailure)
                | (CleanupState::Active, CleanupState::CleanupPending)
                | (CleanupState::CleanupPending, CleanupState::Cleaned)
                | (CleanupState::CleanupPending, CleanupState::CleanupFailed)
                | (CleanupState::CleanupFailed, CleanupState::CleanupPending)
                | (CleanupState::Cleaned, CleanupState::Active)
        );
        if !allowed {
            return Err(RepositoryStateError::InvalidCleanupTransition {
                from: self.cleanup_state,
                to: next,
            });
        }
        if let Some(operation_id) = operation_id {
            if !is_safe_component(operation_id) {
                return Err(RepositoryStateError::InvalidOperationId);
            }
            self.last_workspace_operation_id = Some(operation_id.to_owned());
        } else if matches!(next, CleanupState::Active | CleanupState::RetryableFailure) {
            return Err(RepositoryStateError::OperationIdRequired);
        }
        self.cleanup_state = next;
        self.updated_at = Utc::now();
        self.validate()
    }

    pub fn validate(&self) -> Result<(), RepositoryStateError> {
        if self.schema_version != REPOSITORY_STATE_SCHEMA_VERSION {
            return Err(RepositoryStateError::UnsupportedSchemaVersion {
                actual: self.schema_version,
                supported: REPOSITORY_STATE_SCHEMA_VERSION,
            });
        }
        if !is_safe_component(&self.repository_id) {
            return Err(RepositoryStateError::InvalidRepositoryId);
        }
        if !is_valid_workspace_id(&self.workspace_id) {
            return Err(RepositoryStateError::InvalidWorkspaceId);
        }
        validate_scope_key(&self.workspace_scope_key)?;
        // `RepositoryState::new` retains the legacy scope-only constructor.
        // Every real workspace identity must otherwise derive the stored opaque
        // directory key, preventing a state record from being moved between scopes.
        if self.workspace_id != self.workspace_scope_key
            && derive_workspace_scope_key(&self.workspace_id)? != self.workspace_scope_key
        {
            return Err(RepositoryStateError::InvalidWorkspaceScopeKey);
        }
        if self.updated_at < self.created_at
            || self
                .last_successful_analysis_at
                .is_some_and(|at| at < self.created_at || at > self.updated_at)
            || self
                .last_remote_checked_at
                .is_some_and(|at| at < self.created_at || at > self.updated_at)
        {
            return Err(RepositoryStateError::InvalidTimestampOrder);
        }
        if let Some(operation_id) = &self.last_workspace_operation_id
            && !is_safe_component(operation_id)
        {
            return Err(RepositoryStateError::InvalidOperationId);
        }
        if matches!(
            self.cleanup_state,
            CleanupState::Active
                | CleanupState::RetryableFailure
                | CleanupState::CleanupPending
                | CleanupState::CleanupFailed
        ) && self.last_workspace_operation_id.is_none()
        {
            return Err(RepositoryStateError::OperationIdRequired);
        }
        if let Some(reference) = &self.normalized_ref {
            validate_ref(reference)?;
        }
        if let Some(branch) = &self.default_branch {
            validate_ref(branch)?;
        }
        if let Some(commit) = &self.last_verified_commit {
            validate_commit_id(commit)?;
        }
        if let Some(commit) = &self.last_remote_head {
            validate_commit_id(commit)?;
        }
        if let Some(digest) = &self.last_local_git_state_digest
            && !is_sha256_hex(digest)
        {
            return Err(RepositoryStateError::InvalidLocalGitStateDigest);
        }
        if let Some(snapshot) = &self.last_source_snapshot_id {
            validate_text_identity("source snapshot ID", snapshot)?;
        }
        if let Some(version) = &self.graph_projection_version {
            validate_text_identity("graph projection version", version)?;
        }
        if let Some(verification) = &self.verification {
            verification.validate()?;
            let receipt_matches_state = verification.repository_id == self.repository_id
                && verification.workspace_id == self.workspace_id
                && self.last_verified_commit.as_deref().is_some_and(|commit| {
                    verification.verified_commit.eq_ignore_ascii_case(commit)
                })
                && self.last_verified_commit.as_deref().is_some_and(|commit| {
                    verification
                        .history_head_commit
                        .eq_ignore_ascii_case(commit)
                })
                && self.last_source_snapshot_id.as_deref()
                    == Some(verification.source_snapshot_id.as_str())
                && self.graph_projection_version.as_deref()
                    == Some(verification.graph_projection_version.as_str())
                && self.last_successful_analysis_at.is_some();
            if !receipt_matches_state {
                return Err(RepositoryStateError::InvalidVerificationReceipt);
            }
            if verification.verified_at < self.created_at
                || verification.verified_at > self.updated_at
            {
                return Err(RepositoryStateError::InvalidTimestampOrder);
            }
        }
        if self.completeness == KnowledgeCompleteness::Complete {
            let receipt_is_complete = self.verification.as_ref().is_some_and(|verification| {
                verification.history_complete
                    && verification.history_omitted_commits_at_least == 0
                    && verification.history_omitted_path_changes_at_least == 0
            });
            if self.omitted_file_count != 0 || !receipt_is_complete {
                return Err(RepositoryStateError::CompleteStateHasOmissions);
            }
        }
        if self.analysis_state == AnalysisState::Ready {
            let required = self.last_verified_commit.is_some()
                && self.last_source_snapshot_id.is_some()
                && self.graph_projection_version.is_some()
                && self.verification.is_some()
                && self.completeness != KnowledgeCompleteness::Unknown
                && self.last_successful_analysis_at.is_some();
            if !required {
                return Err(RepositoryStateError::ReadyRequiresVerifiedGraph);
            }
        }
        Ok(())
    }

    pub fn save(&self, paths: &CodeMindPaths) -> Result<(), RepositoryStateError> {
        self.validate()?;
        paths.ensure_repository_scope_dir(&self.repository_id, &self.workspace_scope_key)?;
        let destination =
            paths.repository_state_path(&self.repository_id, &self.workspace_scope_key)?;
        write_json_atomically(&destination, self)
    }

    pub fn load(
        paths: &CodeMindPaths,
        repository_id: &str,
        workspace_scope_key: &str,
    ) -> Result<Self, RepositoryStateError> {
        let expected_scope_key = workspace_scope_key.to_owned();
        validate_scope_key(&expected_scope_key)?;
        let path = paths
            .existing_repository_state_path(repository_id, workspace_scope_key)
            .map_err(map_paths_error)?;
        let mut state: Self = read_json(&path)?;
        migrate_repository_state_schema(&mut state)?;
        state.validate()?;
        if state.repository_id != repository_id || state.workspace_scope_key != expected_scope_key {
            return Err(RepositoryStateError::StatePathIdentityMismatch);
        }
        Ok(state)
    }
}

#[derive(Debug, Error)]
pub enum RepositoryStateError {
    #[error("repository state schema version {actual} is not supported (supported: {supported})")]
    UnsupportedSchemaVersion { actual: u32, supported: u32 },
    #[error(
        "repository state schema {schema_version} contains a value introduced by a later schema"
    )]
    InvalidLegacySchemaPayload { schema_version: u32 },
    #[error("invalid repository ID in repository state")]
    InvalidRepositoryId,
    #[error(
        "invalid workspace-scope key; expected `scope-` plus 64 lowercase hexadecimal characters"
    )]
    InvalidWorkspaceScopeKey,
    #[error("invalid workspace ID in repository state")]
    InvalidWorkspaceId,
    #[error("invalid operation ID in repository state")]
    InvalidOperationId,
    #[error("a workspace operation ID is required while cleanup is active, pending, or failed")]
    OperationIdRequired,
    #[error("invalid repository state transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: AnalysisState,
        to: AnalysisState,
    },
    #[error("invalid cleanup transition from {from:?} to {to:?}")]
    InvalidCleanupTransition {
        from: CleanupState,
        to: CleanupState,
    },
    #[error("repository can only become ready from the verifying state")]
    ReadyRequiresVerificationState,
    #[error("repository cannot be ready without verified commit, snapshot, projection and receipt")]
    ReadyRequiresVerifiedGraph,
    #[error("a repository cannot be marked ready with unknown completeness")]
    UnknownCompletenessForReady,
    #[error("complete repository state cannot report omitted files")]
    CompleteStateHasOmissions,
    #[error("invalid graph-verification receipt")]
    InvalidVerificationReceipt,
    #[error("invalid commit ID; expected 40 or 64 hexadecimal characters")]
    InvalidCommitId,
    #[error("local Git-state digest must be a 64-character SHA-256 hex value")]
    InvalidLocalGitStateDigest,
    #[error("remote freshness and observed HEAD are inconsistent")]
    InconsistentRemoteObservation,
    #[error("invalid {0}; value is empty, too long, or contains control characters")]
    InvalidTextIdentity(&'static str),
    #[error("invalid Git ref/branch value")]
    InvalidRef,
    #[error("repository state timestamps are inconsistent")]
    InvalidTimestampOrder,
    #[error("repository state does not match the requested filesystem identity")]
    StatePathIdentityMismatch,
    #[error("repository state serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("repository state filesystem operation failed at {path:?}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Paths(#[from] CodeMindPathsError),
    #[error(transparent)]
    Identity(#[from] RepositoryIdentityError),
}

fn default_workspace_id() -> String {
    "legacy-workspace".to_owned()
}

fn map_paths_error(error: CodeMindPathsError) -> RepositoryStateError {
    match error {
        CodeMindPathsError::Io { path, source } => RepositoryStateError::Io { path, source },
        other => RepositoryStateError::Paths(other),
    }
}

fn is_valid_workspace_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_scope_key(value: &str) -> Result<(), RepositoryStateError> {
    let suffix = value.strip_prefix("scope-").unwrap_or_default();
    if suffix.len() != 64
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RepositoryStateError::InvalidWorkspaceScopeKey);
    }
    Ok(())
}

fn is_safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn validate_commit_id(value: &str) -> Result<(), RepositoryStateError> {
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(RepositoryStateError::InvalidCommitId);
    }
    Ok(())
}

fn validate_text_identity(kind: &'static str, value: &str) -> Result<(), RepositoryStateError> {
    if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        return Err(RepositoryStateError::InvalidTextIdentity(kind));
    }
    Ok(())
}

fn validate_ref(value: &str) -> Result<(), RepositoryStateError> {
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
        return Err(RepositoryStateError::InvalidRef);
    }
    Ok(())
}

fn write_json_atomically<T: Serialize>(
    destination: &Path,
    value: &T,
) -> Result<(), RepositoryStateError> {
    let parent = destination
        .parent()
        .ok_or(RepositoryStateError::StatePathIdentityMismatch)?;
    let temporary = parent.join(format!(".state-{}.tmp", Uuid::new_v4()));
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
            .map_err(|source| RepositoryStateError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.write_all(&bytes)
            .map_err(|source| RepositoryStateError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.sync_all().map_err(|source| RepositoryStateError::Io {
            path: temporary.clone(),
            source,
        })?;
        fs::rename(&temporary, destination).map_err(|source| RepositoryStateError::Io {
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

/// Upgrade the immediately previous repository-state schema in memory.
/// Version 7 adds a durable cleanup-state enum value but does not change the
/// shape of any version-6 fields, so existing version-6 records can be read
/// safely and are rewritten as version 7 on their next save.
fn migrate_repository_state_schema(
    state: &mut RepositoryState,
) -> Result<(), RepositoryStateError> {
    match state.schema_version {
        PREVIOUS_REPOSITORY_STATE_SCHEMA_VERSION => {
            // `RetryableFailure` was introduced with schema 7. A version-6
            // record containing it is contradictory and must not be upgraded
            // into cleanup authority merely because the new reader knows it.
            if state.cleanup_state == CleanupState::RetryableFailure {
                return Err(RepositoryStateError::InvalidLegacySchemaPayload {
                    schema_version: PREVIOUS_REPOSITORY_STATE_SCHEMA_VERSION,
                });
            }
            state.schema_version = REPOSITORY_STATE_SCHEMA_VERSION;
            Ok(())
        }
        REPOSITORY_STATE_SCHEMA_VERSION => Ok(()),
        actual => Err(RepositoryStateError::UnsupportedSchemaVersion {
            actual,
            supported: REPOSITORY_STATE_SCHEMA_VERSION,
        }),
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, RepositoryStateError> {
    let bytes = fs::read(path).map_err(|source| RepositoryStateError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::{
        AnalysisState, CleanupState, GraphVerificationReceipt, KnowledgeCompleteness,
        REPOSITORY_STATE_SCHEMA_VERSION, RemoteFreshnessState, RepositoryState,
        RepositoryStateError, write_json_atomically,
    };
    use chrono::Utc;

    fn state() -> RepositoryState {
        RepositoryState::new(
            "repo_test",
            "scope-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap()
    }

    fn verified_receipt() -> GraphVerificationReceipt {
        GraphVerificationReceipt {
            verification_id: "verify_123".to_owned(),
            generation_id: "generation_123".to_owned(),
            verified_at: Utc::now(),
            graph_names: vec!["code".to_owned(), "history".to_owned(), "test".to_owned()],
            repository_id: "repo_test".to_owned(),
            workspace_id: "scope-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_owned(),
            source_snapshot_id: "snapshot-a".to_owned(),
            graph_projection_version: "projection-v2".to_owned(),
            verified_commit: "a".repeat(40),
            code_entities: 10,
            code_relationships: 5,
            code_entity_ids_digest: "a".repeat(64),
            code_relationship_ids_digest: "b".repeat(64),
            test_entities: 4,
            test_relationships: 2,
            test_entity_ids_digest: "c".repeat(64),
            test_relationship_ids_digest: "d".repeat(64),
            history_entities: 6,
            history_relationships: 5,
            history_head_commit: "a".repeat(40),
            history_complete: true,
            history_omitted_commits_at_least: 0,
            history_omitted_path_changes_at_least: 0,
            change_entities: 0,
            change_relationships: 0,
        }
    }

    fn advance_to_verifying(state: &mut RepositoryState) {
        state.transition_analysis(AnalysisState::Preparing).unwrap();
        state.transition_analysis(AnalysisState::Analyzing).unwrap();
        state
            .transition_analysis(AnalysisState::Persisting)
            .unwrap();
        state.transition_analysis(AnalysisState::Verifying).unwrap();
    }

    #[test]
    fn new_for_workspace_keeps_workspace_identity_separate_from_scope_key() {
        let workspace_id = "workspace-real";
        // Stable, correctly derived scope key for this fixture. Keeping the
        // value local makes this a state-module test rather than a composition
        // test of the identity module.
        let scope_key = "scope-94394d3bc4de90af3832242dbbd7304a5857dccfa28e9a9649f916b6e056e7d0";
        let state =
            RepositoryState::new_for_workspace("repo_test", workspace_id, scope_key).unwrap();
        assert_eq!(state.workspace_id, workspace_id);
        assert_eq!(state.workspace_scope_key, scope_key);
        assert_ne!(state.workspace_id, state.workspace_scope_key);
    }

    #[test]
    fn new_for_workspace_rejects_a_scope_key_owned_by_another_workspace() {
        let foreign_scope_key =
            "scope-cb5a90118626793b7d7c36cd58e4759f1777c141a832d95a5292552a983f24b6";

        assert!(matches!(
            RepositoryState::new_for_workspace("repo_test", "workspace-real", foreign_scope_key),
            Err(RepositoryStateError::InvalidWorkspaceScopeKey)
        ));
    }

    #[test]
    fn state_serializes_round_trip_with_independent_analysis_and_cleanup_status() {
        let mut state = state();
        advance_to_verifying(&mut state);
        state
            .mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Partial,
                3,
                Some("refs/heads/main"),
            )
            .unwrap();
        state
            .set_cleanup_state(CleanupState::Active, Some("op_123"))
            .unwrap();
        state
            .set_cleanup_state(CleanupState::CleanupPending, None)
            .unwrap();
        state
            .set_cleanup_state(CleanupState::Cleaned, None)
            .unwrap();
        state.validate().unwrap();

        let json = serde_json::to_vec(&state).unwrap();
        let decoded: RepositoryState = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded, state);
        assert_eq!(decoded.analysis_state, AnalysisState::Ready);
        assert_eq!(decoded.cleanup_state, CleanupState::Cleaned);
    }

    #[test]
    fn graph_state_preserves_the_authorized_workspace_scope() {
        let mut state = state();
        advance_to_verifying(&mut state);
        state
            .mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Complete,
                0,
                None,
            )
            .unwrap();

        let graph_state = state.graph_state().unwrap().unwrap();
        assert_eq!(graph_state.workspace_id, state.workspace_id);
        assert_eq!(graph_state.verification.workspace_id, state.workspace_id);
        assert!(graph_state.is_usable());
    }

    #[test]
    fn graph_state_rejects_receipt_scope_mismatch() {
        let mut state = state();
        advance_to_verifying(&mut state);
        let mut receipt = verified_receipt();
        receipt.workspace_id = "other-workspace".to_owned();
        assert!(
            state
                .mark_ready(
                    &"a".repeat(40),
                    "snapshot-a",
                    "projection-v2",
                    "generation_123",
                    receipt,
                    KnowledgeCompleteness::Complete,
                    0,
                    None,
                )
                .is_err()
        );
    }

    #[test]
    fn ready_cannot_be_entered_without_verifying_and_receipt() {
        let mut state = state();
        assert!(matches!(
            state.mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Complete,
                0,
                None,
            ),
            Err(RepositoryStateError::ReadyRequiresVerificationState)
        ));

        advance_to_verifying(&mut state);
        assert!(
            state
                .mark_ready(
                    &"a".repeat(40),
                    "snapshot-a",
                    "projection-v2",
                    "generation_123",
                    verified_receipt(),
                    KnowledgeCompleteness::Unknown,
                    0,
                    None,
                )
                .is_err()
        );
        assert_eq!(state.analysis_state, AnalysisState::Verifying);
    }

    #[test]
    fn complete_state_cannot_claim_omitted_files() {
        let mut state = state();
        advance_to_verifying(&mut state);
        assert!(matches!(
            state.mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Complete,
                1,
                None,
            ),
            Err(RepositoryStateError::CompleteStateHasOmissions)
        ));
    }

    #[test]
    fn cleanup_failure_requires_a_recorded_pending_state() {
        let mut state = state();
        state
            .set_cleanup_state(CleanupState::Active, Some("op_direct_failure"))
            .unwrap();
        assert!(matches!(
            state.set_cleanup_state(CleanupState::CleanupFailed, None),
            Err(RepositoryStateError::InvalidCleanupTransition { .. })
        ));
        state
            .set_cleanup_state(CleanupState::CleanupPending, None)
            .unwrap();
        state
            .set_cleanup_state(CleanupState::CleanupFailed, None)
            .unwrap();
    }

    #[test]
    fn cleanup_failure_does_not_erase_ready_graph_state() {
        let mut state = state();
        advance_to_verifying(&mut state);
        state
            .mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Complete,
                0,
                Some("refs/heads/main"),
            )
            .unwrap();
        state
            .set_cleanup_state(CleanupState::Active, Some("op_456"))
            .unwrap();
        state
            .set_cleanup_state(CleanupState::CleanupPending, None)
            .unwrap();
        state
            .set_cleanup_state(CleanupState::CleanupFailed, None)
            .unwrap();

        assert_eq!(state.analysis_state, AnalysisState::Ready);
        assert_eq!(state.cleanup_state, CleanupState::CleanupFailed);
        assert!(state.last_verified_commit.is_some());
    }

    #[test]
    fn unknown_remote_observation_preserves_last_known_head() {
        let mut state = state();
        let known = "a".repeat(40);
        state
            .record_remote_observation(Some(&known), RemoteFreshnessState::Unchanged, Utc::now())
            .unwrap();
        state
            .record_remote_observation(None, RemoteFreshnessState::Unknown, Utc::now())
            .unwrap();
        assert_eq!(state.last_remote_head.as_deref(), Some(known.as_str()));
        assert_eq!(state.last_remote_freshness, RemoteFreshnessState::Unknown);
    }

    #[test]
    fn remote_observation_requires_a_head_for_known_freshness_and_none_for_unknown() {
        let mut state = state();
        let head = "a".repeat(40);
        assert!(matches!(
            state.record_remote_observation(None, RemoteFreshnessState::Unchanged, Utc::now()),
            Err(RepositoryStateError::InconsistentRemoteObservation)
        ));
        assert!(matches!(
            state.record_remote_observation(Some(&head), RemoteFreshnessState::Unknown, Utc::now()),
            Err(RepositoryStateError::InconsistentRemoteObservation)
        ));
        assert!(
            state
                .record_remote_observation(Some(&head), RemoteFreshnessState::Changed, Utc::now())
                .is_ok()
        );
    }

    #[test]
    fn local_git_state_digest_is_validated_and_normalized() {
        let mut state = state();
        let digest = "A".repeat(64);
        state.record_local_git_state_digest(Some(&digest)).unwrap();
        let normalized = "a".repeat(64);
        assert_eq!(
            state.last_local_git_state_digest.as_deref(),
            Some(normalized.as_str())
        );
        assert!(matches!(
            state.record_local_git_state_digest(Some("not-a-digest")),
            Err(RepositoryStateError::InvalidLocalGitStateDigest)
        ));
        state.record_local_git_state_digest(None).unwrap();
        assert_eq!(state.last_local_git_state_digest, None);
    }

    #[test]
    fn complete_state_rejects_incomplete_history_receipts() {
        let mut state = state();
        advance_to_verifying(&mut state);
        let mut receipt = verified_receipt();
        receipt.history_complete = false;
        receipt.history_omitted_path_changes_at_least = 1;
        let result = state.mark_ready(
            &"a".repeat(40),
            "snapshot-a",
            "projection-v2",
            "generation_123",
            receipt,
            KnowledgeCompleteness::Complete,
            0,
            None,
        );
        assert!(matches!(
            result,
            Err(RepositoryStateError::CompleteStateHasOmissions)
        ));
        assert_eq!(state.analysis_state, AnalysisState::Verifying);
    }

    #[test]
    fn complete_state_remains_valid_for_complete_source_and_history() {
        let mut state = state();
        advance_to_verifying(&mut state);
        state
            .mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Complete,
                0,
                None,
            )
            .unwrap();
        assert_eq!(state.completeness, KnowledgeCompleteness::Complete);
        assert!(state.validate().is_ok());
    }

    #[test]
    fn ready_state_exposes_a_usable_graph_state_without_requiring_source_files() {
        let mut state = state();
        advance_to_verifying(&mut state);
        state
            .mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Complete,
                0,
                None,
            )
            .unwrap();

        let graph = state.graph_state().unwrap().expect("ready graph state");
        assert!(graph.is_usable());
        assert!(graph.matches_commit(&"A".repeat(40)));
        assert_eq!(graph.repository_id, "repo_test");
        assert_eq!(graph.source_snapshot_id, "snapshot-a");
    }

    #[test]
    fn non_ready_state_does_not_expose_graph_state() {
        let state = state();
        assert!(state.graph_state().unwrap().is_none());
    }

    #[test]
    fn partial_graph_state_remains_usable_when_verified() {
        let mut state = state();
        advance_to_verifying(&mut state);
        state
            .mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Partial,
                2,
                None,
            )
            .unwrap();

        assert!(state.graph_state().unwrap().unwrap().is_usable());
    }

    #[test]
    fn receipt_requires_sha256_identity_digests_and_accepts_opaque_workspace_ids() {
        let mut receipt = verified_receipt();
        receipt.workspace_id = "tenant:project:123".to_owned();
        assert!(receipt.validate().is_ok());

        receipt.code_entity_ids_digest = "not-a-digest".to_owned();
        assert!(matches!(
            receipt.validate(),
            Err(RepositoryStateError::InvalidVerificationReceipt)
        ));
    }

    #[test]
    fn ready_state_rejects_receipts_for_another_repository_scope_or_commit() {
        let mut state = state();
        advance_to_verifying(&mut state);
        state
            .mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Complete,
                0,
                None,
            )
            .unwrap();

        state.verification.as_mut().unwrap().workspace_id = "other-scope".to_owned();
        assert!(matches!(
            state.validate(),
            Err(RepositoryStateError::InvalidVerificationReceipt)
        ));
    }

    #[test]
    fn active_or_pending_cleanup_state_requires_its_operation_owner() {
        let mut state = state();
        state.cleanup_state = CleanupState::Active;
        assert!(matches!(
            state.validate(),
            Err(RepositoryStateError::OperationIdRequired)
        ));

        state.cleanup_state = CleanupState::CleanupPending;
        assert!(matches!(
            state.validate(),
            Err(RepositoryStateError::OperationIdRequired)
        ));
    }

    #[test]
    fn retryable_failure_keeps_repository_non_ready_and_requires_its_owner() {
        let mut state = state();
        state.transition_analysis(AnalysisState::Preparing).unwrap();
        state
            .set_cleanup_state(CleanupState::Active, Some("operation_retry"))
            .unwrap();
        state
            .set_cleanup_state(CleanupState::RetryableFailure, Some("operation_retry"))
            .unwrap();

        assert_eq!(state.cleanup_state, CleanupState::RetryableFailure);
        assert_eq!(state.analysis_state, AnalysisState::Preparing);
        assert!(state.graph_state().unwrap().is_none());

        let mut invalid = state.clone();
        invalid.last_workspace_operation_id = None;
        assert!(matches!(
            invalid.validate(),
            Err(RepositoryStateError::OperationIdRequired)
        ));
    }

    #[test]
    fn schema_and_transition_errors_are_explicit() {
        let mut state = state();
        assert!(matches!(
            state.transition_analysis(AnalysisState::Ready),
            Err(RepositoryStateError::InvalidTransition { .. })
        ));
        state.schema_version = 999;
        assert!(matches!(
            state.validate(),
            Err(RepositoryStateError::UnsupportedSchemaVersion { .. })
        ));
    }

    #[test]
    fn previous_repository_state_schema_is_upgraded_without_changing_state() {
        let mut state = state();
        state.schema_version = super::PREVIOUS_REPOSITORY_STATE_SCHEMA_VERSION;
        let repository_id = state.repository_id.clone();
        let workspace_scope_key = state.workspace_scope_key.clone();

        super::migrate_repository_state_schema(&mut state).unwrap();

        assert_eq!(state.schema_version, REPOSITORY_STATE_SCHEMA_VERSION);
        assert_eq!(state.repository_id, repository_id);
        assert_eq!(state.workspace_scope_key, workspace_scope_key);
        assert_eq!(state.cleanup_state, CleanupState::NotCreated);
        state.validate().unwrap();

        let mut future = state.clone();
        future.schema_version = REPOSITORY_STATE_SCHEMA_VERSION + 1;
        assert!(matches!(
            super::migrate_repository_state_schema(&mut future),
            Err(RepositoryStateError::UnsupportedSchemaVersion { .. })
        ));
        assert_eq!(future.schema_version, REPOSITORY_STATE_SCHEMA_VERSION + 1);

        let mut mixed_previous = state.clone();
        mixed_previous.schema_version = super::PREVIOUS_REPOSITORY_STATE_SCHEMA_VERSION;
        mixed_previous.cleanup_state = CleanupState::RetryableFailure;
        assert!(matches!(
            super::migrate_repository_state_schema(&mut mixed_previous),
            Err(RepositoryStateError::InvalidLegacySchemaPayload { schema_version: 6 })
        ));
    }

    #[test]
    fn failed_atomic_state_write_does_not_create_a_ready_state_record() {
        use std::fs;
        use tempfile::TempDir;

        let mut ready = state();
        advance_to_verifying(&mut ready);
        ready
            .mark_ready(
                &"a".repeat(40),
                "snapshot-a",
                "projection-v2",
                "generation_123",
                verified_receipt(),
                KnowledgeCompleteness::Complete,
                0,
                None,
            )
            .unwrap();

        let directory = TempDir::new().unwrap();
        let destination = directory.path().join("state.json");
        // A directory at the destination forces the final rename to fail on
        // supported platforms. The temporary JSON must not appear as a file.
        fs::create_dir(&destination).unwrap();

        assert!(write_json_atomically(&destination, &ready).is_err());
        assert!(destination.is_dir());
        let remaining_entries = fs::read_dir(directory.path()).unwrap().count();
        assert_eq!(
            remaining_entries, 1,
            "failed writes must remove their temp file"
        );
    }

    #[test]
    fn state_deserialization_rejects_unknown_fields_for_its_schema_version() {
        let json = serde_json::json!({
            "schema_version": REPOSITORY_STATE_SCHEMA_VERSION,
            "repository_id": "repo_test",
            "workspace_scope_key": "scope-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "workspace_id": "scope-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "analysis_state": "unknown",
            "cleanup_state": "not_created",
            "last_workspace_operation_id": null,
            "normalized_ref": null,
            "default_branch": null,
            "last_verified_commit": null,
            "last_source_snapshot_id": null,
            "graph_projection_version": null,
            "verification": null,
            "completeness": "unknown",
            "omitted_file_count": 0,
            "last_remote_head": null,
            "last_remote_checked_at": null,
            "last_remote_freshness": "unknown",
            "last_local_git_state_digest": null,
            "last_successful_analysis_at": null,
            "created_at": Utc::now(),
            "updated_at": Utc::now(),
            "unrecognized": true
        });
        assert!(serde_json::from_value::<RepositoryState>(json).is_err());
    }
}
