//! Recovery for operation-owned temporary repository workspaces.
//!
//! Graph-operation event replay remains in `lifecycle::recovery`. This module
//! owns only workspace filesystem recovery and deliberately separates:
//! verified graph publication, retryable indexing failures, and cleanup state.

use crate::{
    graph_storage::{GraphStorage, GraphVerificationExpectation},
    repository::{
        AnalysisState, CleanupState, CodeMindPaths, GraphVerificationReceipt, RepositoryState,
        RepositoryStateError, WorkspaceCleanupReason, WorkspaceManager, WorkspaceMarker,
        WorkspacePhase,
    },
    source::RepositoryContext,
};
use std::{io, sync::Arc};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphProof {
    Verified,
    NotVerified,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryDecision {
    CleanupVerified,
    CleanupAbandoned,
    RetainForRetry,
    PreserveForManualReview,
    SkipBecauseActive,
}

fn same_workspace_ownership_identity(
    scanned: &crate::repository::WorkspaceMarker,
    current: &crate::repository::WorkspaceMarker,
) -> bool {
    scanned.operation_id == current.operation_id
        && scanned.repository_id == current.repository_id
        && scanned.workspace_scope_key == current.workspace_scope_key
        && scanned.remote_locator == current.remote_locator
        && scanned.requested_ref == current.requested_ref
        && scanned.workspace_root == current.workspace_root
        && scanned.repository_root == current.repository_root
        && scanned.owner == current.owner
        && scanned.owned_temporary_checkout == current.owned_temporary_checkout
}

fn decide_recovery(
    state_ready: bool,
    marker_can_be_retained: bool,
    receipt_matches_workspace_head: bool,
    graph_proof: GraphProof,
    marker_already_claims_verified_cleanup: bool,
    scope_lock_acquired: bool,
) -> RecoveryDecision {
    if !scope_lock_acquired {
        return RecoveryDecision::SkipBecauseActive;
    }
    if !marker_can_be_retained {
        return RecoveryDecision::PreserveForManualReview;
    }
    if state_ready {
        // Ready is a durable claim that must be backed by the same workspace
        // head and a fresh storage proof. Any missing, unavailable, or
        // contradictory evidence requires manual review, never auto-retry.
        return if receipt_matches_workspace_head && graph_proof == GraphProof::Verified {
            RecoveryDecision::CleanupVerified
        } else {
            RecoveryDecision::PreserveForManualReview
        };
    }
    if marker_already_claims_verified_cleanup {
        return RecoveryDecision::PreserveForManualReview;
    }
    RecoveryDecision::RetainForRetry
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceRecoveryReport {
    pub scanned: usize,
    pub cleaned_verified: usize,
    pub cleaned_abandoned: usize,
    pub retained_for_retry: usize,
    pub skipped_active: usize,
    pub preserved_for_manual_review: usize,
    pub issues: Vec<WorkspaceRecoveryIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecoveryIssue {
    pub operation_id: String,
    pub code: &'static str,
}

#[derive(Debug, Error)]
pub enum WorkspaceRecoveryError {
    #[error("workspace recovery could not scan the canonical workspace root: {0}")]
    Workspace(#[from] crate::repository::WorkspaceError),
    #[error("workspace recovery could not prepare the canonical CodeMind root: {0}")]
    Paths(#[from] crate::repository::CodeMindPathsError),
    #[error("repository state recovery failed: {0}")]
    State(#[from] RepositoryStateError),
    #[error("cannot abandon a retryable workspace while its repository scope is active")]
    ScopeBusy,
}

/// Coordinates startup recovery of workspace markers with durable repository
/// state and a fresh graph-storage verification.
pub struct WorkspaceRecovery<S> {
    storage: Arc<S>,
    paths: CodeMindPaths,
    workspaces: WorkspaceManager,
}

impl<S> WorkspaceRecovery<S>
where
    S: GraphStorage + 'static,
{
    pub fn new(storage: Arc<S>, paths: CodeMindPaths) -> Self {
        Self {
            storage,
            workspaces: WorkspaceManager::new(paths.clone()),
            paths,
        }
    }

    /// Recover abandoned workspaces without touching any currently locked scope.
    ///
    /// Incomplete operations are retained as retryable. A workspace is deleted
    /// only if its exact repository state and active generation still verify.
    pub async fn recover_startup(&self) -> Result<WorkspaceRecoveryReport, WorkspaceRecoveryError> {
        let candidates = self.workspaces.scan_owned_workspaces()?;
        let mut report = WorkspaceRecoveryReport {
            scanned: candidates.len(),
            ..WorkspaceRecoveryReport::default()
        };

        for candidate in candidates {
            let lock = match self.paths.try_acquire_repository_scope_lock(
                &candidate.repository_id,
                &candidate.workspace_scope_key,
            ) {
                Ok(Some(lock)) => lock,
                Ok(None) => {
                    report.skipped_active += 1;
                    continue;
                }
                Err(_) => {
                    report.issues.push(WorkspaceRecoveryIssue {
                        operation_id: candidate.operation_id,
                        code: "scope_lock_failed",
                    });
                    continue;
                }
            };

            // The scan result can become stale while waiting for the lock.
            let marker = match self.workspaces.load_marker(&candidate.operation_id) {
                Ok(marker) => marker,
                Err(_) => {
                    report.issues.push(WorkspaceRecoveryIssue {
                        operation_id: candidate.operation_id,
                        code: "marker_changed_or_invalid",
                    });
                    drop(lock);
                    continue;
                }
            };
            if !same_workspace_ownership_identity(&candidate, &marker) {
                report.preserved_for_manual_review += 1;
                report.issues.push(WorkspaceRecoveryIssue {
                    operation_id: marker.operation_id.clone(),
                    code: "marker_identity_changed_after_scan",
                });
                drop(lock);
                continue;
            }

            let mut state = match RepositoryState::load(
                &self.paths,
                &marker.repository_id,
                &marker.workspace_scope_key,
            ) {
                Ok(state) => Some(state),
                Err(RepositoryStateError::Io { source, .. })
                    if source.kind() == io::ErrorKind::NotFound =>
                {
                    None
                }
                Err(_) => {
                    report.preserved_for_manual_review += 1;
                    report.issues.push(WorkspaceRecoveryIssue {
                        operation_id: marker.operation_id.clone(),
                        code: "repository_state_unavailable",
                    });
                    drop(lock);
                    continue;
                }
            };

            let state_owns_marker = state.as_ref().is_some_and(|state| {
                state.repository_id == marker.repository_id
                    && state.workspace_scope_key == marker.workspace_scope_key
                    && state.last_workspace_operation_id.as_deref()
                        == Some(marker.operation_id.as_str())
            });
            let state_ready = state
                .as_ref()
                .is_some_and(|state| state.analysis_state == AnalysisState::Ready);
            let receipt = state.as_ref().and_then(|state| state.verification.as_ref());
            let receipt_matches_workspace_head = receipt.is_some_and(|receipt| {
                marker
                    .head_commit
                    .as_deref()
                    .is_some_and(|head| receipt.verified_commit.eq_ignore_ascii_case(head))
            });
            let cleanup_claim = marker.cleanup_reason
                == Some(WorkspaceCleanupReason::GraphsVerified)
                && matches!(
                    marker.phase,
                    WorkspacePhase::CleanupPending | WorkspacePhase::CleanupFailed
                );

            let graph_proof = if state_ready && state_owns_marker && receipt_matches_workspace_head
            {
                match self
                    .verify_receipt(state.as_ref().expect("ready state was checked"))
                    .await
                {
                    Ok(true) => GraphProof::Verified,
                    Ok(false) => GraphProof::NotVerified,
                    Err(_) => GraphProof::Unavailable,
                }
            } else {
                GraphProof::NotVerified
            };

            // A valid CodeMind marker may predate its first durable state
            // write (process interruption immediately after workspace creation).
            // Such a marker can be retained for retry, but can never authorize
            // deletion. A present state record with a different operation owner
            // remains a hard ownership mismatch.
            let state_can_retain_marker = state_owns_marker
                && state
                    .as_ref()
                    .is_some_and(|state| state.cleanup_state != CleanupState::Cleaned);
            let marker_may_be_retained = state.is_none()
                || state.as_ref().is_some_and(|state| {
                    state.cleanup_state == CleanupState::NotCreated
                        && state.last_workspace_operation_id.is_none()
                })
                || state_can_retain_marker;

            // Explicit abandonment is a separate cleanup authorization from
            // graph verification. It is honored only when the marker records
            // that explicit choice and any present state record still assigns
            // ownership to this exact operation.
            let explicitly_abandoned = marker.cleanup_reason
                == Some(WorkspaceCleanupReason::RetryAbandoned)
                && matches!(
                    marker.phase,
                    WorkspacePhase::CleanupPending | WorkspacePhase::CleanupFailed
                )
                && (state.is_none()
                    || (state_owns_marker
                        && state.as_ref().is_some_and(|state| {
                            matches!(
                                state.cleanup_state,
                                CleanupState::CleanupPending | CleanupState::CleanupFailed
                            )
                        })));
            let decision = if explicitly_abandoned {
                RecoveryDecision::CleanupAbandoned
            } else {
                decide_recovery(
                    state_ready,
                    marker_may_be_retained,
                    receipt_matches_workspace_head,
                    graph_proof,
                    cleanup_claim,
                    true,
                )
            };

            match decision {
                RecoveryDecision::CleanupVerified => {
                    let mut state = state.expect("verified graph requires repository state");
                    match self.cleanup_verified_workspace(&marker, &mut state).await {
                        Ok(true) => report.cleaned_verified += 1,
                        Ok(false) => report.preserved_for_manual_review += 1,
                        Err(_) => {
                            report.preserved_for_manual_review += 1;
                            report.issues.push(WorkspaceRecoveryIssue {
                                operation_id: marker.operation_id.clone(),
                                code: "verified_cleanup_retry_failed",
                            });
                        }
                    }
                }
                RecoveryDecision::CleanupAbandoned => {
                    match self
                        .workspaces
                        .retry_abandoned_cleanup(&marker.operation_id)
                    {
                        Ok(()) => {
                            if let Some(state) = state.as_mut() {
                                if state.cleanup_state == CleanupState::CleanupFailed {
                                    let _ = state.set_cleanup_state(
                                        CleanupState::CleanupPending,
                                        Some(&marker.operation_id),
                                    );
                                }
                                if state.cleanup_state == CleanupState::CleanupPending {
                                    let _ = state.set_cleanup_state(
                                        CleanupState::Cleaned,
                                        Some(&marker.operation_id),
                                    );
                                }
                                let _ = state.save(&self.paths);
                            }
                            report.cleaned_abandoned += 1;
                        }
                        Err(_) => {
                            if let Some(state) = state.as_mut()
                                && state.cleanup_state == CleanupState::CleanupPending
                            {
                                let _ = state.set_cleanup_state(
                                    CleanupState::CleanupFailed,
                                    Some(&marker.operation_id),
                                );
                                let _ = state.save(&self.paths);
                            }
                            report.preserved_for_manual_review += 1;
                            report.issues.push(WorkspaceRecoveryIssue {
                                operation_id: marker.operation_id.clone(),
                                code: "abandoned_cleanup_retry_failed",
                            });
                        }
                    }
                }
                RecoveryDecision::RetainForRetry => {
                    if self.mark_retryable(&marker, state).is_ok() {
                        report.retained_for_retry += 1;
                    } else {
                        report.preserved_for_manual_review += 1;
                        report.issues.push(WorkspaceRecoveryIssue {
                            operation_id: marker.operation_id.clone(),
                            code: "could_not_record_retryable_failure",
                        });
                    }
                }
                RecoveryDecision::PreserveForManualReview => {
                    report.preserved_for_manual_review += 1;
                    report.issues.push(WorkspaceRecoveryIssue {
                        operation_id: marker.operation_id.clone(),
                        code: "verification_or_ownership_not_proven",
                    });
                }
                RecoveryDecision::SkipBecauseActive => {
                    report.skipped_active += 1;
                }
            }
            drop(lock);
        }

        Ok(report)
    }

    /// Explicitly abandon a retryable workspace. The caller must make this
    /// decision; startup recovery never deletes an unverified clone merely
    /// because it is old or has failed repeatedly.
    pub async fn abandon_retryable_workspace(
        &self,
        operation_id: &str,
    ) -> Result<(), WorkspaceRecoveryError> {
        let marker = self.workspaces.load_marker(operation_id)?;
        let lock = match self
            .paths
            .try_acquire_repository_scope_lock(&marker.repository_id, &marker.workspace_scope_key)?
        {
            Some(lock) => lock,
            None => return Err(WorkspaceRecoveryError::ScopeBusy),
        };
        let current = self.workspaces.load_marker(operation_id)?;
        if !same_workspace_ownership_identity(&marker, &current) {
            return Err(crate::repository::WorkspaceError::MarkerIdentityMismatch.into());
        }
        if current.phase != WorkspacePhase::RetryableFailure {
            return Err(crate::repository::WorkspaceError::RetryNotAllowed(current.phase).into());
        }

        let mut state = match RepositoryState::load(
            &self.paths,
            &current.repository_id,
            &current.workspace_scope_key,
        ) {
            Ok(state) => Some(state),
            Err(RepositoryStateError::Io { source, .. })
                if source.kind() == io::ErrorKind::NotFound =>
            {
                None
            }
            Err(_error) => {
                return Err(WorkspaceRecoveryError::Workspace(
                    crate::repository::WorkspaceError::InvalidMarker,
                ));
            }
        };

        if let Some(state) = state.as_mut() {
            if state.last_workspace_operation_id.as_deref() != Some(operation_id) {
                return Err(crate::repository::WorkspaceError::MarkerIdentityMismatch.into());
            }
            match state.cleanup_state {
                CleanupState::RetryableFailure | CleanupState::Active => {
                    state
                        .set_cleanup_state(CleanupState::CleanupPending, Some(operation_id))
                        .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
                }
                CleanupState::NotCreated => {
                    state
                        .set_cleanup_state(CleanupState::Active, Some(operation_id))
                        .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
                    state
                        .set_cleanup_state(CleanupState::CleanupPending, Some(operation_id))
                        .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
                }
                _ => return Err(crate::repository::WorkspaceError::CleanupReasonMismatch.into()),
            }
            state
                .save(&self.paths)
                .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
        }

        if let Err(error) = self.workspaces.abandon_retryable_workspace(operation_id) {
            if let Some(state) = state.as_mut()
                && state.cleanup_state == CleanupState::CleanupPending
            {
                let _ = state.set_cleanup_state(CleanupState::CleanupFailed, Some(operation_id));
                let _ = state.save(&self.paths);
            }
            return Err(error.into());
        }
        if let Some(state) = state.as_mut() {
            state
                .set_cleanup_state(CleanupState::Cleaned, Some(operation_id))
                .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
            state
                .save(&self.paths)
                .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
        }
        drop(lock);
        Ok(())
    }

    async fn verify_receipt(
        &self,
        state: &RepositoryState,
    ) -> Result<bool, crate::graph_storage::StorageError> {
        let Some(graph_state) = state
            .graph_state()
            .map_err(|error| crate::graph_storage::StorageError::Query(error.to_string()))?
        else {
            return Ok(false);
        };
        if !graph_state.is_usable() {
            return Ok(false);
        }
        let receipt = &graph_state.verification;
        let context = RepositoryContext {
            repository_id: graph_state.repository_id,
            workspace_id: graph_state.workspace_id,
            source_snapshot_id: graph_state.source_snapshot_id,
            graph_version: graph_state.graph_projection_version,
        };
        let Some(expected) = expectation_from_receipt(&context, receipt) else {
            return Ok(false);
        };
        if !self
            .storage
            .verify_published_generation(&context, &receipt.generation_id)
            .await?
        {
            return Ok(false);
        }
        let report = self.storage.verify_graph(&context, &expected).await?;
        Ok(report.satisfies(&expected))
    }

    async fn cleanup_verified_workspace(
        &self,
        marker: &WorkspaceMarker,
        state: &mut RepositoryState,
    ) -> Result<bool, crate::repository::WorkspaceError> {
        self.workspaces.mark_verified_after_recovery(
            &marker.operation_id,
            marker
                .head_commit
                .as_deref()
                .ok_or(crate::repository::WorkspaceError::VerifiedWorkspaceHasNoHead)?,
        )?;

        match state.cleanup_state {
            CleanupState::NotCreated => {
                state
                    .set_cleanup_state(CleanupState::Active, Some(&marker.operation_id))
                    .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
                state
                    .set_cleanup_state(CleanupState::CleanupPending, Some(&marker.operation_id))
                    .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
            }
            CleanupState::Active | CleanupState::RetryableFailure | CleanupState::CleanupFailed => {
                state
                    .set_cleanup_state(CleanupState::CleanupPending, Some(&marker.operation_id))
                    .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
            }
            CleanupState::CleanupPending => {}
            CleanupState::Cleaned => return Ok(false),
        }
        state
            .save(&self.paths)
            .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;

        match self
            .workspaces
            .cleanup_after_verification(&marker.operation_id)
        {
            Ok(()) => {
                state
                    .set_cleanup_state(CleanupState::Cleaned, Some(&marker.operation_id))
                    .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
                state
                    .save(&self.paths)
                    .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
                Ok(true)
            }
            Err(error) => {
                let _ = state
                    .set_cleanup_state(CleanupState::CleanupFailed, Some(&marker.operation_id));
                let _ = state.save(&self.paths);
                Err(error)
            }
        }
    }

    fn mark_retryable(
        &self,
        marker: &WorkspaceMarker,
        mut state: Option<RepositoryState>,
    ) -> Result<(), crate::repository::WorkspaceError> {
        self.workspaces
            .mark_retryable_failure(&marker.operation_id, "startup_interrupted")?;
        let Some(state) = state.as_mut() else {
            return Ok(());
        };
        if state.last_workspace_operation_id.as_deref() != Some(marker.operation_id.as_str()) {
            return Ok(());
        }
        if matches!(
            state.analysis_state,
            AnalysisState::Preparing
                | AnalysisState::Analyzing
                | AnalysisState::Persisting
                | AnalysisState::Verifying
        ) {
            state
                .transition_analysis(AnalysisState::Failed)
                .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
        }
        match state.cleanup_state {
            CleanupState::NotCreated => {
                state
                    .set_cleanup_state(CleanupState::Active, Some(&marker.operation_id))
                    .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
                state
                    .set_cleanup_state(CleanupState::RetryableFailure, Some(&marker.operation_id))
                    .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
            }
            CleanupState::Active | CleanupState::CleanupPending | CleanupState::CleanupFailed => {
                state
                    .set_cleanup_state(CleanupState::RetryableFailure, Some(&marker.operation_id))
                    .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
            }
            CleanupState::RetryableFailure => {}
            CleanupState::Cleaned => return Ok(()),
        }
        state
            .save(&self.paths)
            .map_err(|_| crate::repository::WorkspaceError::InvalidMarker)?;
        Ok(())
    }
}

fn expectation_from_receipt(
    context: &RepositoryContext,
    receipt: &GraphVerificationReceipt,
) -> Option<GraphVerificationExpectation> {
    Some(GraphVerificationExpectation {
        context: context.clone(),
        generation_id: receipt.generation_id.clone(),
        code_entities: usize::try_from(receipt.code_entities).ok()?,
        code_relationships: usize::try_from(receipt.code_relationships).ok()?,
        code_entity_ids_digest: receipt.code_entity_ids_digest.clone(),
        code_relationship_ids_digest: receipt.code_relationship_ids_digest.clone(),
        test_entities: usize::try_from(receipt.test_entities).ok()?,
        test_relationships: usize::try_from(receipt.test_relationships).ok()?,
        test_entity_ids_digest: receipt.test_entity_ids_digest.clone(),
        test_relationship_ids_digest: receipt.test_relationship_ids_digest.clone(),
        history_entities: usize::try_from(receipt.history_entities).ok()?,
        history_relationships: usize::try_from(receipt.history_relationships).ok()?,
        history_head_commit: receipt.history_head_commit.clone(),
        history_complete: receipt.history_complete,
        history_omitted_commits_at_least: receipt.history_omitted_commits_at_least,
        history_omitted_path_changes_at_least: receipt.history_omitted_path_changes_at_least,
        change_entities: usize::try_from(receipt.change_entities).ok()?,
        change_relationships: usize::try_from(receipt.change_relationships).ok()?,
    })
}

#[cfg(test)]
mod level1_tests {
    use super::{GraphProof, RecoveryDecision, decide_recovery};

    #[test]
    fn recovery_deletes_only_when_scope_receipt_and_storage_are_verified() {
        assert_eq!(
            decide_recovery(true, true, true, GraphProof::Verified, false, true),
            RecoveryDecision::CleanupVerified
        );
        assert_eq!(
            decide_recovery(true, true, true, GraphProof::Unavailable, false, true),
            RecoveryDecision::PreserveForManualReview
        );
        assert_eq!(
            decide_recovery(true, true, true, GraphProof::NotVerified, false, true),
            RecoveryDecision::PreserveForManualReview
        );
        assert_eq!(
            decide_recovery(true, true, false, GraphProof::Verified, false, true),
            RecoveryDecision::PreserveForManualReview
        );
    }

    #[test]
    fn interrupted_unverified_operations_are_retained_for_retry() {
        assert_eq!(
            decide_recovery(false, true, false, GraphProof::NotVerified, false, true),
            RecoveryDecision::RetainForRetry
        );
        assert_eq!(
            decide_recovery(false, true, false, GraphProof::NotVerified, false, false),
            RecoveryDecision::SkipBecauseActive
        );
    }

    #[test]
    fn valid_orphan_marker_without_a_state_record_is_retained_for_retry() {
        // The marker itself was validated against the canonical workspace
        // directory. Absence of a state file is enough to retain it, never to
        // authorize destructive cleanup.
        assert_eq!(
            decide_recovery(false, true, false, GraphProof::NotVerified, false, true),
            RecoveryDecision::RetainForRetry
        );
    }

    #[test]
    fn legacy_cleanup_claims_without_current_state_are_never_blindly_deleted() {
        assert_eq!(
            decide_recovery(false, true, false, GraphProof::NotVerified, true, true),
            RecoveryDecision::PreserveForManualReview
        );
        assert_eq!(
            decide_recovery(false, false, false, GraphProof::NotVerified, false, true),
            RecoveryDecision::PreserveForManualReview
        );
    }
}
