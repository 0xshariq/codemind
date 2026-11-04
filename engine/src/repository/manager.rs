//! Repository lifecycle coordinator for durable indexing and graph reuse.

use crate::repository::{
    AnalysisState, CleanupState, CodeMindPaths, CodeMindPathsError, GraphVerificationReceipt,
    KnowledgeCompleteness, RemoteFreshnessState, RepositoryIdentity, RepositoryMetadata,
    RepositoryState, RepositoryStateError, WorkspaceManager, WorkspaceMarker, WorkspacePhase,
    WorkspaceRecovery, WorkspaceRecoveryError, WorkspaceRecoveryReport, WorkspaceRetryMode,
    identity::workspace_scope_key,
};
use crate::{
    contract::result::Freshness,
    engines::repository_index::{RepositoryIndexError, RepositoryIndexPipeline},
    git::{self, GitCommandConfig, GitHistoryOptions, GitRepository},
    graph_storage::{GraphStorage, StorageError},
    lifecycle::Operation,
    source::{
        RepositoryContext, SourceProvider, SourceSnapshot, identity::graph_version_for_snapshot,
    },
};
use std::{fs, io, path::PathBuf, sync::Arc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug)]
pub struct PreparedRepository {
    pub snapshot: SourceSnapshot,
    pub git_repository: GitRepository,
    pub git_history: git::GitHistory,
    /// Present only for caller-owned local checkouts. Remote clones are owned
    /// temporary workspaces and are freshness-checked against the remote ref.
    pub local_git_state_digest: Option<String>,
    /// Some only for caller-owned local checkouts. Only `Some(true)` proves
    /// that commit-only incremental mutation is safe; a dirty or unobservable
    /// local tree is conservatively represented as `Some(false)`.
    pub local_git_worktree_clean: Option<bool>,
    pub owned_operation_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RepositoryKnowledge {
    pub context: RepositoryContext,
    pub selected_entity_id: String,
    pub reused: bool,
    /// Compatibility summary. `completeness` carries the explicit three-state
    /// value used by the investigation pipeline.
    pub complete: bool,
    pub completeness: KnowledgeCompleteness,
    /// Freshness is independent from graph completeness and cleanup outcome.
    pub freshness: Freshness,
    pub omitted_file_count: u64,
    pub history_complete: bool,
    pub history_omitted_commits_at_least: u64,
    pub history_omitted_path_changes_at_least: u64,
    pub cleanup_state: CleanupState,
    pub head_commit: String,
    /// Exact source manifest when it is available from the current capture.
    /// Reused knowledge may not have a source tree, so the transport layer may
    /// fall back to repository-level metadata for older state records.
    pub manifest_json: Option<String>,
}

#[derive(Debug, Error)]
pub enum RepositoryManagerError {
    #[error("invalid repository request: {0}")]
    InvalidRequest(String),
    #[error("repository identity error: {0}")]
    Identity(#[from] crate::repository::RepositoryIdentityError),
    #[error("repository state error: {0}")]
    State(#[from] RepositoryStateError),
    #[error("workspace error: {0}")]
    Workspace(#[from] crate::repository::WorkspaceError),
    #[error("Git error: {0}")]
    Git(#[from] git::GitError),
    #[error("source provider error: {0}")]
    Source(#[from] crate::source::SourceProviderError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("indexing error: {0}")]
    Indexing(#[from] RepositoryIndexError),
    #[error("operation error: {0}")]
    Operation(String),
    #[error("selected symbol or file was not found")]
    SelectedEntityNotFound,
    #[error("selected symbol or file is ambiguous")]
    SelectedEntityAmbiguous,
    #[error("repository state file is unavailable or requires manual recovery")]
    StateUnavailable,
    #[error("CodeMind path or repository-scope lock error: {0}")]
    Paths(#[from] CodeMindPathsError),
    #[error("workspace recovery error: {0}")]
    Recovery(#[from] WorkspaceRecoveryError),
}

#[derive(Debug, Clone, Default)]
pub struct RepositoryManagerOptions {
    pub history_options: GitHistoryOptions,
    pub enable_incremental_mutation: bool,
}

pub struct RepositoryManager<S> {
    storage: Arc<S>,
    paths: CodeMindPaths,
    workspaces: WorkspaceManager,
    provider: SourceProvider,
    git_config: GitCommandConfig,
    options: RepositoryManagerOptions,
}

/// Best-effort crash/early-return marker guard for an owned temporary clone.
/// The workspace itself is never deleted by Drop.
struct RetryableWorkspaceGuard {
    workspaces: WorkspaceManager,
    operation_id: String,
    armed: bool,
}

impl RetryableWorkspaceGuard {
    fn new(workspaces: WorkspaceManager, operation_id: String) -> Self {
        Self {
            workspaces,
            operation_id,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for RetryableWorkspaceGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self
                .workspaces
                .mark_retryable_failure(&self.operation_id, "operation_interrupted");
        }
    }
}

impl<S> RepositoryManager<S>
where
    S: GraphStorage + 'static,
{
    pub fn new(
        storage: Arc<S>,
        paths: CodeMindPaths,
        provider: SourceProvider,
        git_config: GitCommandConfig,
    ) -> Self {
        Self::with_options(
            storage,
            paths,
            provider,
            git_config,
            RepositoryManagerOptions::default(),
        )
    }

    pub fn with_options(
        storage: Arc<S>,
        paths: CodeMindPaths,
        provider: SourceProvider,
        git_config: GitCommandConfig,
        options: RepositoryManagerOptions,
    ) -> Self {
        let workspaces = WorkspaceManager::new(paths.clone());
        Self {
            storage,
            paths,
            workspaces,
            provider,
            git_config,
            options,
        }
    }

    pub fn paths(&self) -> &CodeMindPaths {
        &self.paths
    }

    /// Recover operation-owned workspaces before accepting new repository work.
    ///
    /// Graph-operation event replay remains a separate lifecycle concern. This
    /// coordinator reconciles only temporary workspace markers with repository
    /// state and a fresh storage-generation verification.
    pub async fn recover_startup(&self) -> Result<WorkspaceRecoveryReport, RepositoryManagerError> {
        WorkspaceRecovery::new(Arc::clone(&self.storage), self.paths.clone())
            .recover_startup()
            .await
            .map_err(Into::into)
    }

    /// Explicitly abandon one retained retryable clone. This is destructive,
    /// so the recovery coordinator revalidates its marker, scope lock and
    /// repository-state ownership before removing anything.
    pub async fn abandon_retryable_workspace(
        &self,
        operation_id: &str,
    ) -> Result<(), RepositoryManagerError> {
        WorkspaceRecovery::new(Arc::clone(&self.storage), self.paths.clone())
            .abandon_retryable_workspace(operation_id)
            .await
            .map_err(Into::into)
    }

    /// Ensure repository-level Code/History/Test knowledge is ready without
    /// making repository resolution depend on a question-specific symbol.
    /// This is the shared lifecycle entry point for the repository gRPC service.
    #[allow(clippy::too_many_arguments)]
    pub async fn ensure_repository_ready(
        &self,
        source_locator: &str,
        workspace_id: &str,
        requested_ref: Option<&str>,
        operation_id: &str,
        request_id: &str,
        trace_id: &str,
    ) -> Result<RepositoryKnowledge, RepositoryManagerError> {
        self.ensure_indexed_internal(
            source_locator,
            workspace_id,
            requested_ref,
            None,
            operation_id,
            request_id,
            trace_id,
        )
        .await
    }

    /// Ensure the repository graph exists and is verified for this workspace
    /// scope, then resolve one exact selected symbol/file from the persisted
    /// Code Graph. Remote repositories use a lightweight ref check before the
    /// reuse path; inaccessible remotes conservatively fall back to indexing.
    #[allow(clippy::too_many_arguments)]
    pub async fn ensure_indexed(
        &self,
        source_locator: &str,
        workspace_id: &str,
        requested_ref: Option<&str>,
        symbol_name: &str,
        operation_id: &str,
        request_id: &str,
        trace_id: &str,
    ) -> Result<RepositoryKnowledge, RepositoryManagerError> {
        self.ensure_indexed_internal(
            source_locator,
            workspace_id,
            requested_ref,
            Some(symbol_name),
            operation_id,
            request_id,
            trace_id,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn ensure_indexed_internal(
        &self,
        source_locator: &str,
        workspace_id: &str,
        requested_ref: Option<&str>,
        symbol_name: Option<&str>,
        operation_id: &str,
        request_id: &str,
        trace_id: &str,
    ) -> Result<RepositoryKnowledge, RepositoryManagerError> {
        validate_request_component("source_locator", source_locator)?;
        validate_request_component("workspace_id", workspace_id)?;
        if let Some(symbol_name) = symbol_name {
            validate_request_component("symbol_name", symbol_name)?;
        }
        validate_request_component("operation_id", operation_id)?;
        validate_request_component("request_id", request_id)?;
        validate_request_component("trace_id", trace_id)?;

        let (identity, local_path) = self.resolve_identity(source_locator).await?;
        let scope_key = workspace_scope_key(workspace_id)?;

        // The lock covers the complete read/decide/index/verify/publish/state
        // sequence. It is process-wide, not merely an in-memory mutex, and is
        // keyed by the authorized workspace/repository scope. Blocking file
        // locking runs off the async executor.
        let lock_paths = self.paths.clone();
        let lock_repository_id = identity.repository_id().to_owned();
        let lock_scope_key = scope_key.clone();
        let _scope_lock = tokio::task::spawn_blocking(move || {
            lock_paths.acquire_repository_scope_lock(&lock_repository_id, &lock_scope_key)
        })
        .await
        .map_err(|error| {
            RepositoryManagerError::Operation(format!(
                "repository-scope lock worker failed: {error}"
            ))
        })??;

        let mut metadata = match RepositoryMetadata::load(&self.paths, identity.repository_id()) {
            Ok(metadata) => metadata,
            Err(crate::repository::RepositoryIdentityError::Io { source, .. })
                if source.kind() == io::ErrorKind::NotFound =>
            {
                RepositoryMetadata::new(identity.clone())?
            }
            Err(error) => return Err(error.into()),
        };
        metadata.touch();
        metadata.save(&self.paths)?;

        let mut pipeline_decision =
            crate::engines::pipeline::RepositoryPipelineDecision::Initialize;

        if let Some(mut state) =
            load_state_optional(&self.paths, identity.repository_id(), &scope_key)?.filter(
                |state| {
                    state.analysis_state == AnalysisState::Ready
                        && state.completeness != KnowledgeCompleteness::Unknown
                        && state.last_verified_commit.is_some()
                        && state.last_source_snapshot_id.is_some()
                        && state.graph_projection_version.is_some()
                        && state.verification.is_some()
                },
            )
        {
            let graph_state = state.graph_state()?;
            let graph_matches = if let Some(graph_state) = graph_state.as_ref() {
                let context = context_from_state(&state)?;
                let projection_matches = graph_state.graph_projection_version
                    == graph_version_for_snapshot(&graph_state.source_snapshot_id);
                let expected = verification_expectation(&state);
                let published = if let Some(receipt) = state.verification.as_ref() {
                    self.storage
                        .verify_published_generation(&context, &receipt.generation_id)
                        .await
                        .unwrap_or(false)
                } else {
                    false
                };
                published
                    && self
                        .storage
                        .verify_graph(&context, &expected)
                        .await
                        .map(|report| report.satisfies(&expected))
                        .unwrap_or(false)
                    && projection_matches
                    && graph_state.is_usable()
                    && state.completeness != KnowledgeCompleteness::Unknown
            } else {
                false
            };

            // Determine the current commit without capturing source files. For
            // remotes this is a lightweight ref observation; for local roots it
            // is only a Git metadata read. Any inability to establish the
            // current commit is conservative and therefore leads to Update.
            let current_commit = if let Some(local_path) = local_path.as_ref() {
                // Local inputs are caller-owned and may have staged, unstaged,
                // untracked, renamed or deleted files without changing HEAD.
                // Re-capture the source contract and compare the independent
                // Git-state fingerprint before allowing graph reuse.
                match git::open_local_repository(local_path, &self.git_config).await {
                    Ok(repository) => {
                        match repository.local_state_observation(&self.git_config).await {
                            Ok(local_git_state) => {
                                match self.provider.resolve_with_repository_identity(
                                    local_path,
                                    workspace_id.to_owned(),
                                    Some(identity.repository_id()),
                                ) {
                                    Ok(snapshot)
                                        if local_snapshot_matches_verified_state(
                                            &snapshot.context.source_snapshot_id,
                                            &snapshot.manifest.source_snapshot_id,
                                            snapshot.manifest.omitted_files.len() as u64,
                                            state
                                                .last_source_snapshot_id
                                                .as_deref()
                                                .unwrap_or_default(),
                                            state.omitted_file_count,
                                            Some(&local_git_state.digest),
                                            state.last_local_git_state_digest.as_deref(),
                                        ) =>
                                    {
                                        Some(repository.head_commit().to_owned())
                                    }
                                    Ok(_) => {
                                        tracing::debug!(
                                            repository_id = %identity.repository_id(),
                                            "local source or Git index state differs from the verified capture; forcing update"
                                        );
                                        None
                                    }
                                    Err(error) => {
                                        tracing::warn!(
                                            repository_id = %identity.repository_id(),
                                            error = %error,
                                            "could not capture local source state before reuse decision; forcing update"
                                        );
                                        None
                                    }
                                }
                            }
                            Err(error) => {
                                tracing::warn!(
                                    repository_id = %identity.repository_id(),
                                    error = %error,
                                    "could not fingerprint local Git state before reuse decision; forcing update"
                                );
                                None
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            repository_id = %identity.repository_id(),
                            error = %error,
                            "could not open local Git state before reuse decision; forcing update"
                        );
                        None
                    }
                }
            } else if identity.kind() == crate::repository::RepositoryIdentityKind::Remote {
                if let Some(locator) = identity.remote_locator() {
                    let observation = git::observe_remote_freshness(
                        locator,
                        requested_ref,
                        state.last_verified_commit.as_deref().unwrap(),
                        &self.git_config,
                    )
                    .await;
                    match observation {
                        Ok(observation) => {
                            match (observation.freshness, observation.remote_head.as_ref()) {
                                (git::RemoteFreshness::Unknown, _) | (_, None) => {
                                    state.record_remote_observation(
                                        None,
                                        RemoteFreshnessState::Unknown,
                                        observation
                                            .remote_head
                                            .as_ref()
                                            .map(|head| head.checked_at)
                                            .unwrap_or_else(chrono::Utc::now),
                                    )?;
                                    state.save(&self.paths)?;
                                    None
                                }
                                (freshness, Some(remote_head)) => {
                                    let remote_state = match freshness {
                                        git::RemoteFreshness::Unchanged => {
                                            RemoteFreshnessState::Unchanged
                                        }
                                        git::RemoteFreshness::Changed => {
                                            RemoteFreshnessState::Changed
                                        }
                                        git::RemoteFreshness::Unknown => unreachable!(),
                                    };
                                    state.record_remote_observation(
                                        Some(&remote_head.commit_id),
                                        remote_state,
                                        remote_head.checked_at,
                                    )?;
                                    state.save(&self.paths)?;
                                    if matches!(freshness, git::RemoteFreshness::Unchanged) {
                                        Some(remote_head.commit_id.clone())
                                    } else {
                                        None
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                repository_id = %identity.repository_id(),
                                error = %error,
                                "remote freshness could not be proven; treating it as unknown"
                            );
                            state.record_remote_observation(
                                None,
                                RemoteFreshnessState::Unknown,
                                chrono::Utc::now(),
                            )?;
                            state.save(&self.paths)?;
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let decision = crate::engines::pipeline::decide_repository_pipeline(
                graph_state.is_some(),
                graph_matches,
                state.last_verified_commit.as_deref(),
                current_commit.as_deref(),
            );
            pipeline_decision = decision;

            if matches!(
                decision,
                crate::engines::pipeline::RepositoryPipelineDecision::Reuse
            ) {
                let context = context_from_state(&state)?;
                let selected = match symbol_name {
                    Some(symbol_name) => {
                        self.resolve_selected_entity(&context, symbol_name).await?
                    }
                    None => String::new(),
                };
                let mut operation =
                    Operation::new(operation_id, request_id, trace_id, context.clone())
                        .map_err(RepositoryManagerError::Operation)?;
                self.storage.persist_operation(&operation.record).await?;
                let event = operation
                    .begin_stage("repository-reuse")
                    .map_err(RepositoryManagerError::Operation)?;
                self.storage.persist_operation(&operation.record).await?;
                self.storage
                    .append_operation_event(&crate::contract::events::EngineEvent::Progress {
                        metadata: event.metadata.clone(),
                        engine: "codemind".to_owned(),
                        stage: "repository-reuse".to_owned(),
                        message:
                            "repository pipeline gate selected reuse; verified knowledge is current"
                                .to_owned(),
                    })
                    .await?;
                let event = operation
                    .finish_stage("repository-reuse")
                    .map_err(RepositoryManagerError::Operation)?;
                self.storage.persist_operation(&operation.record).await?;
                self.storage
                    .append_operation_event(&crate::contract::events::EngineEvent::Progress {
                        metadata: event.metadata.clone(),
                        engine: "codemind".to_owned(),
                        stage: "repository-reuse".to_owned(),
                        message: "verified repository knowledge reused".to_owned(),
                    })
                    .await?;
                if should_complete_repository_operation(symbol_name) {
                    self.complete_repository_only_operation(&mut operation)
                        .await?;
                }
                let verification = state
                    .verification
                    .as_ref()
                    .ok_or(RepositoryManagerError::StateUnavailable)?;
                return Ok(RepositoryKnowledge {
                    context,
                    selected_entity_id: selected,
                    reused: true,
                    complete: state.completeness == KnowledgeCompleteness::Complete,
                    completeness: state.completeness,
                    freshness: Freshness::Current,
                    omitted_file_count: state.omitted_file_count,
                    history_complete: verification.history_complete,
                    history_omitted_commits_at_least: verification.history_omitted_commits_at_least,
                    history_omitted_path_changes_at_least: verification
                        .history_omitted_path_changes_at_least,
                    cleanup_state: state.cleanup_state,
                    head_commit: state.last_verified_commit.unwrap(),
                    manifest_json: None,
                });
            }
        }

        let operation_id_owned = operation_id.to_owned();
        let mut workspace_failure_guard: Option<RetryableWorkspaceGuard> = None;
        let (prepared, mut state, requested_ref) = if let Some(local_path) = local_path {
            let git_repository = git::open_local_repository(&local_path, &self.git_config).await?;
            let stable_identity = RepositoryIdentity::from_local_directory_with_origin(
                &local_path,
                git_repository.remote_locator(),
            )?;
            if stable_identity.repository_id() != identity.repository_id() {
                return Err(RepositoryManagerError::InvalidRequest(
                    "local repository identity changed while preparing the request".to_owned(),
                ));
            }
            let snapshot = self.provider.resolve_with_repository_identity(
                &local_path,
                workspace_id.to_owned(),
                Some(stable_identity.repository_id()),
            )?;
            let local_git_state = match git_repository
                .local_state_observation(&self.git_config)
                .await
            {
                Ok(observation) => Some(observation),
                Err(error) => {
                    tracing::warn!(
                        repository_id = %identity.repository_id(),
                        error = %error,
                        "local Git-state fingerprint is unavailable; indexing will be conservative and freshness will remain unknown"
                    );
                    None
                }
            };
            let history = git::read_history(
                &git_repository,
                self.options.history_options,
                &self.git_config,
            )
            .await?;
            let state = prepare_state(
                &self.paths,
                identity.repository_id(),
                workspace_id,
                &scope_key,
            )?;
            (
                PreparedRepository {
                    snapshot,
                    git_repository,
                    git_history: history,
                    local_git_state_digest: local_git_state
                        .as_ref()
                        .map(|observation| observation.digest.clone()),
                    local_git_worktree_clean: Some(
                        local_git_state
                            .as_ref()
                            .is_some_and(|observation| observation.working_tree_clean),
                    ),
                    owned_operation_id: None,
                },
                state,
                requested_ref.map(str::to_owned),
            )
        } else {
            let mut state = prepare_state(
                &self.paths,
                identity.repository_id(),
                workspace_id,
                &scope_key,
            )?;
            let (workspace_operation_id, retry_mode) = self.prepare_owned_workspace(
                &mut state,
                &identity,
                &scope_key,
                &operation_id_owned,
                requested_ref,
            )?;

            // From this point every early return or propagated error leaves a
            // retryable marker instead of deleting the source tree or leaving
            // an ambiguous in-progress marker. It is disarmed only once the
            // graph and durable state are verified.
            workspace_failure_guard = Some(RetryableWorkspaceGuard::new(
                self.workspaces.clone(),
                workspace_operation_id.clone(),
            ));

            let git_repository = match retry_mode {
                WorkspaceRetryMode::CloneAgain => {
                    git::GitRepository::clone_remote(
                        &self.workspaces,
                        &workspace_operation_id,
                        &self.git_config,
                    )
                    .await
                }
                WorkspaceRetryMode::ResumeExistingClone => {
                    git::GitRepository::resume_retryable_workspace(
                        &self.workspaces,
                        &workspace_operation_id,
                        &self.git_config,
                    )
                    .await
                }
            };
            let git_repository = match git_repository {
                Ok(repository) => repository,
                Err(error) => {
                    let _ = mark_failed_state(&self.paths, &mut state, &workspace_operation_id);
                    self.record_retryable_workspace_failure(
                        &mut state,
                        &workspace_operation_id,
                        "clone_or_resume_failed",
                    );
                    return Err(error.into());
                }
            };
            let snapshot = match self.provider.resolve_with_repository_identity(
                git_repository.root(),
                workspace_id.to_owned(),
                Some(identity.repository_id()),
            ) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    let _ = mark_failed_state(&self.paths, &mut state, &workspace_operation_id);
                    self.record_retryable_workspace_failure(
                        &mut state,
                        &workspace_operation_id,
                        "source_capture_failed",
                    );
                    return Err(error.into());
                }
            };
            let history = match git::read_history(
                &git_repository,
                self.options.history_options,
                &self.git_config,
            )
            .await
            {
                Ok(history) => history,
                Err(error) => {
                    let _ = mark_failed_state(&self.paths, &mut state, &workspace_operation_id);
                    self.record_retryable_workspace_failure(
                        &mut state,
                        &workspace_operation_id,
                        "history_failed",
                    );
                    return Err(error.into());
                }
            };
            (
                PreparedRepository {
                    snapshot,
                    git_repository,
                    git_history: history,
                    local_git_state_digest: None,
                    local_git_worktree_clean: None,
                    owned_operation_id: Some(workspace_operation_id),
                },
                state,
                requested_ref.map(str::to_owned),
            )
        };

        state.transition_analysis(AnalysisState::Analyzing)?;
        state.save(&self.paths)?;
        let mut operation = Operation::new(
            operation_id,
            request_id,
            trace_id,
            prepared.snapshot.context.clone(),
        )
        .map_err(RepositoryManagerError::Operation)?;
        self.storage.persist_operation(&operation.record).await?;

        let mut incremental_diff: Option<git::GitDiff> = None;
        let mut incremental_previous_context: Option<crate::source::RepositoryContext> = None;
        if matches!(
            pipeline_decision,
            crate::engines::pipeline::RepositoryPipelineDecision::Update
        ) && let Some(base_commit) = state.last_verified_commit.clone()
        {
            let stage = operation
                .begin_stage("repository-change-detection")
                .map_err(RepositoryManagerError::Operation)?;
            self.storage.persist_operation(&operation.record).await?;
            self.storage
                .append_operation_event(&crate::contract::events::EngineEvent::Progress {
                    metadata: stage.metadata.clone(),
                    engine: "codemind".to_owned(),
                    stage: "repository-change-detection".to_owned(),
                    message: "comparing the last verified commit with the prepared repository"
                        .to_owned(),
                })
                .await?;

            let (detection_message, detected_diff) = match git::diff_commits(
                &prepared.git_repository,
                &base_commit,
                prepared.git_repository.head_commit(),
                git::GitDiffOptions::default(),
                &self.git_config,
            )
            .await
            {
                Ok(diff) => {
                    let strategy = if crate::engines::repository_index::INCREMENTAL_MUTATION_PRODUCTION_ENABLED
                        && self.options.enable_incremental_mutation
                        && crate::engines::pipeline::incremental_path_mapping_is_safe(&diff)
                    {
                        crate::engines::pipeline::select_repository_update_strategy(Some(diff.complete))
                    } else {
                        crate::engines::pipeline::RepositoryUpdateStrategy::FullRebuild
                    };
                    match strategy {
                        crate::engines::pipeline::RepositoryUpdateStrategy::IncrementalMutation => {
                            let previous_history = state.verification.as_ref();
                            let previous_graph_is_complete = incremental_update_base_is_complete(
                                state.completeness == KnowledgeCompleteness::Complete,
                                state.omitted_file_count,
                                previous_history.is_some_and(|receipt| receipt.history_complete),
                                previous_history.map_or(0, |receipt| receipt.history_omitted_commits_at_least),
                                previous_history.map_or(0, |receipt| receipt.history_omitted_path_changes_at_least),
                                !prepared.snapshot.manifest.omitted_files.is_empty(),
                                prepared.git_history.complete,
                                prepared.git_history.omitted_commits_at_least,
                                history_path_omissions_at_least(
                                    prepared.git_history.commits.iter().map(|commit| {
                                        (
                                            commit.changed_paths_complete,
                                            commit.omitted_paths_at_least,
                                        )
                                    }),
                                ),
                                prepared.local_git_worktree_clean,
                            );
                            if previous_graph_is_complete {
                                if let Ok(Some(graph_state)) = state.graph_state() {
                                    incremental_previous_context = Some(
                                        crate::source::RepositoryContext {
                                            repository_id: graph_state.repository_id,
                                            workspace_id: graph_state.workspace_id,
                                            source_snapshot_id: graph_state.source_snapshot_id,
                                            graph_version: graph_state.graph_projection_version,
                                        },
                                    );
                                    (
                                        format!(
                                            "detected {} complete Git path changes; applying incremental graph mutation",
                                            diff.changes.len()
                                        ),
                                        Some(diff),
                                    )
                                } else {
                                    (
                                        "verified graph context is unavailable; using full rebuild".to_owned(),
                                        None,
                                    )
                                }
                            } else if prepared.local_git_worktree_clean == Some(false) {
                                (
                                    "caller-owned local checkout has staged, unstaged, or untracked changes; commit-only diff cannot cover them, so using a full rebuild".to_owned(),
                                    None,
                                )
                            } else {
                                (
                                    "previous or current repository knowledge is partial; using full rebuild to avoid promoting partial graph facts".to_owned(),
                                    None,
                                )
                            }
                        },
                        crate::engines::pipeline::RepositoryUpdateStrategy::FullRebuild => (
                            if prepared.local_git_worktree_clean == Some(false) {
                                "caller-owned local checkout has staged, unstaged, or untracked changes; commit-only diff cannot cover them, so using a full rebuild".to_owned()
                            } else if !diff.complete {
                                format!(
                                    "Git change set is incomplete (at least {} path changes omitted); using full rebuild",
                                    diff.omitted_change_count
                                )
                            } else if diff.changes.is_empty() {
                                "the source commit changed without a path-level diff; rebuilding the repository index and history safely".to_owned()
                            } else {
                                "Git change paths cannot be mapped losslessly to the source snapshot; using full rebuild".to_owned()
                            },
                            None,
                        ),
                    }
                }
                Err(git::GitError::BaseCommitUnavailable { .. })
                | Err(git::GitError::BaseCommitNotAncestor { .. }) => (
                    "verified Git history base is unavailable or no longer an ancestor; using full rebuild"
                        .to_owned(),
                    None,
                ),
                Err(_) => (
                    "Git change detection is unavailable; using full rebuild rather than assuming an empty diff"
                        .to_owned(),
                    None,
                ),
            };
            incremental_diff = detected_diff;

            let finished = operation
                .finish_stage("repository-change-detection")
                .map_err(RepositoryManagerError::Operation)?;
            self.storage.persist_operation(&operation.record).await?;
            self.storage
                .append_operation_event(&crate::contract::events::EngineEvent::Progress {
                    metadata: finished.metadata.clone(),
                    engine: "codemind".to_owned(),
                    stage: "repository-change-detection".to_owned(),
                    message: detection_message,
                })
                .await?;
        }

        let indexer = RepositoryIndexPipeline::new(Arc::clone(&self.storage));
        state.transition_analysis(AnalysisState::Persisting)?;
        state.save(&self.paths)?;
        if let Some(owned_operation_id) = prepared.owned_operation_id.as_deref() {
            // Keep the durable workspace marker aligned with the repository
            // lifecycle. Successful cleanup is allowed only after the marker
            // reaches `Verified`, which itself is reached only after the
            // graph-storage verification stage below.
            self.workspaces.transition(
                owned_operation_id,
                crate::repository::WorkspacePhase::Persisting,
            )?;
        }
        let result = match if let (Some(previous_context), Some(diff)) = (
            incremental_previous_context.as_ref(),
            incremental_diff.as_ref(),
        ) {
            indexer
                .run_incremental(previous_context, &prepared, diff, &mut operation)
                .await
        } else {
            indexer.run(&prepared, &mut operation).await
        } {
            Ok(result) => result,
            Err(error) => {
                state.transition_analysis(AnalysisState::Failed)?;
                operation.record.error_code = Some("REPOSITORY_INDEX_FAILED".to_owned());
                let _ = operation.transition(crate::contract::operation::OperationState::Failed);
                let _ = self.storage.persist_operation(&operation.record).await;
                if prepared.owned_operation_id.is_some() {
                    self.record_retryable_workspace_failure(
                        &mut state,
                        &operation_id_owned,
                        "index_failed",
                    );
                }
                return Err(error.into());
            }
        };

        state.transition_analysis(AnalysisState::Verifying)?;
        state.save(&self.paths)?;
        if let Some(owned_operation_id) = prepared.owned_operation_id.as_deref() {
            self.workspaces.transition(
                owned_operation_id,
                crate::repository::WorkspacePhase::Verifying,
            )?;
        }
        let freshness = self
            .record_prepared_source_freshness(
                &identity,
                workspace_id,
                requested_ref.as_deref(),
                &prepared,
                &mut state,
            )
            .await?;
        state.save(&self.paths)?;

        let generation_id =
            crate::graph_storage::generation_id_for_context(&result.verification.context);
        self.storage
            .publish_verified_generation(&result.verification.context, &generation_id)
            .await?;
        if !self
            .storage
            .verify_published_generation(&result.verification.context, &generation_id)
            .await?
        {
            return Err(RepositoryManagerError::Storage(StorageError::Query(
                "storage publication marker did not match the verified generation".to_owned(),
            )));
        }
        let retention_preview = self
            .storage
            .prune_obsolete_graph_projections(&result.verification.context, &generation_id, true)
            .await?;
        if retention_preview.eligible_code_entities > 0
            || retention_preview.eligible_test_entities > 0
        {
            // Do not delete an older projection while an investigation may
            // still be reading it. The storage contract does not yet expose
            // durable in-flight snapshot leases, so pruning remains preview-only.
            tracing::warn!(
                eligible_code_entities = retention_preview.eligible_code_entities,
                eligible_test_entities = retention_preview.eligible_test_entities,
                "obsolete graph projections retained until in-flight investigation protection is available"
            );
        }

        let receipt = GraphVerificationReceipt {
            verification_id: format!("verification-{}", Uuid::new_v4()),
            generation_id: generation_id.clone(),
            verified_at: chrono::Utc::now(),
            graph_names: vec!["code".into(), "history".into(), "test".into()],
            repository_id: result.verification.context.repository_id.clone(),
            workspace_id: result.verification.context.workspace_id.clone(),
            source_snapshot_id: result.verification.context.source_snapshot_id.clone(),
            graph_projection_version: result.verification.context.graph_version.clone(),
            verified_commit: result.head_commit.clone(),
            code_entities: result.verification.code_entities as u64,
            code_relationships: result.verification.code_relationships as u64,
            code_entity_ids_digest: result.verification.code_entity_ids_digest.clone(),
            code_relationship_ids_digest: result.verification.code_relationship_ids_digest.clone(),
            test_entities: result.verification.test_entities as u64,
            test_relationships: result.verification.test_relationships as u64,
            test_entity_ids_digest: result.verification.test_entity_ids_digest.clone(),
            test_relationship_ids_digest: result.verification.test_relationship_ids_digest.clone(),
            history_entities: result.verification.history_entities as u64,
            history_relationships: result.verification.history_relationships as u64,
            history_head_commit: result.verification.history_head_commit.clone(),
            history_complete: result.verification.history_complete,
            history_omitted_commits_at_least: result.verification.history_omitted_commits_at_least,
            history_omitted_path_changes_at_least: result
                .verification
                .history_omitted_path_changes_at_least,
            change_entities: result.verification.change_entities as u64,
            change_relationships: result.verification.change_relationships as u64,
        };
        let completeness = if result.complete {
            KnowledgeCompleteness::Complete
        } else {
            KnowledgeCompleteness::Partial
        };
        state.mark_ready(
            &result.head_commit,
            &prepared.snapshot.context.source_snapshot_id,
            &prepared.snapshot.context.graph_version,
            &generation_id,
            receipt,
            completeness,
            result.omitted_file_count,
            requested_ref
                .as_deref()
                .or_else(|| prepared.git_repository.branch()),
        )?;
        state.save(&self.paths)?;

        if let Some(owned_operation_id) = prepared.owned_operation_id.as_deref() {
            // The verified graph is durable and Ready at this point. Disarm the
            // pre-verification failure guard before any cleanup bookkeeping: a
            // cleanup failure must never downgrade a verified graph to a
            // retryable analysis failure.
            if let Some(guard) = workspace_failure_guard.as_mut() {
                guard.disarm();
            }

            let cleanup_pending_persisted = match state
                .set_cleanup_state(CleanupState::CleanupPending, Some(owned_operation_id))
            {
                Ok(()) => match state.save(&self.paths) {
                    Ok(()) => true,
                    Err(error) => {
                        tracing::warn!(
                            operation_id = %owned_operation_id,
                            error = %error,
                            "verified graph is ready but cleanup-pending state could not be persisted"
                        );
                        let _ = state.set_cleanup_state(
                            CleanupState::CleanupFailed,
                            Some(owned_operation_id),
                        );
                        let _ = state.save(&self.paths);
                        false
                    }
                },
                Err(error) => {
                    tracing::warn!(
                        operation_id = %owned_operation_id,
                        error = %error,
                        "verified graph is ready but cleanup could not enter its pending state"
                    );
                    false
                }
            };

            if cleanup_pending_persisted {
                match self.workspaces.transition(
                    owned_operation_id,
                    crate::repository::WorkspacePhase::Verified,
                ) {
                    Ok(_) => match self
                        .workspaces
                        .cleanup_after_verification(owned_operation_id)
                    {
                        Ok(()) => {
                            if let Err(error) = state
                                .set_cleanup_state(CleanupState::Cleaned, Some(owned_operation_id))
                                .and_then(|_| state.save(&self.paths))
                            {
                                // The workspace is already absent. A later
                                // request reconciles the durable pending state
                                // from that absence instead of recreating it.
                                tracing::warn!(
                                    operation_id = %owned_operation_id,
                                    error = %error,
                                    "workspace was removed but cleaned state could not be persisted"
                                );
                            }
                        }
                        Err(error) => {
                            let _ = state.set_cleanup_state(
                                CleanupState::CleanupFailed,
                                Some(owned_operation_id),
                            );
                            let _ = state.save(&self.paths);
                            tracing::warn!(
                                operation_id = %owned_operation_id,
                                error = %error,
                                "repository graph is ready but temporary workspace cleanup failed"
                            );
                        }
                    },
                    Err(error) => {
                        let _ = state.set_cleanup_state(
                            CleanupState::CleanupFailed,
                            Some(owned_operation_id),
                        );
                        let _ = state.save(&self.paths);
                        tracing::warn!(
                            operation_id = %owned_operation_id,
                            error = %error,
                            "repository graph is ready but its workspace marker could not be advanced to verified cleanup"
                        );
                    }
                }
            }
        }

        let selected = match symbol_name {
            Some(symbol_name) => {
                self.resolve_selected_entity(&prepared.snapshot.context, symbol_name)
                    .await?
            }
            None => String::new(),
        };
        if should_complete_repository_operation(symbol_name) {
            self.complete_repository_only_operation(&mut operation)
                .await?;
        }
        Ok(RepositoryKnowledge {
            context: prepared.snapshot.context.clone(),
            selected_entity_id: selected,
            reused: false,
            complete: result.complete,
            completeness: if result.complete {
                KnowledgeCompleteness::Complete
            } else {
                KnowledgeCompleteness::Partial
            },
            freshness,
            omitted_file_count: result.omitted_file_count,
            history_complete: result.history_complete,
            history_omitted_commits_at_least: result.history_omitted_commits_at_least,
            history_omitted_path_changes_at_least: result.history_omitted_path_changes_at_least,
            cleanup_state: state.cleanup_state,
            head_commit: result.head_commit,
            manifest_json: Some(serde_json::to_string(&prepared.snapshot.manifest).map_err(
                |error| {
                    RepositoryManagerError::Operation(format!(
                        "failed to serialize repository manifest: {error}"
                    ))
                },
            )?),
        })
    }

    /// Revalidate the source that was indexed before publishing its verified
    /// generation. This is an observation only: local directories remain
    /// caller-owned, while remote state is queried without retaining a clone.
    async fn record_prepared_source_freshness(
        &self,
        identity: &RepositoryIdentity,
        workspace_id: &str,
        requested_ref: Option<&str>,
        prepared: &PreparedRepository,
        state: &mut RepositoryState,
    ) -> Result<Freshness, RepositoryManagerError> {
        if prepared.owned_operation_id.is_none() {
            if prepared.local_git_state_digest.is_none() {
                state.record_local_git_state_digest(None)?;
                return Ok(Freshness::Unknown);
            }
            // Caller-owned local repository: both the source manifest and the
            // staged/unstaged Git fingerprint must still match the captured
            // input. If revalidation itself fails, preserve Unknown rather
            // than claiming Current.
            let freshness = match git::open_local_repository(
                prepared.git_repository.root(),
                &self.git_config,
            )
            .await
            {
                Ok(repository) => {
                    match repository.local_state_observation(&self.git_config).await {
                        Ok(current_git_state) => {
                            match self.provider.resolve_with_repository_identity(
                                prepared.git_repository.root(),
                                workspace_id.to_owned(),
                                Some(identity.repository_id()),
                            ) {
                                Ok(current_snapshot)
                                    if repository.head_commit()
                                        == prepared.git_repository.head_commit()
                                        && local_snapshot_matches_verified_state(
                                            &current_snapshot.context.source_snapshot_id,
                                            &current_snapshot.manifest.source_snapshot_id,
                                            current_snapshot.manifest.omitted_files.len() as u64,
                                            &prepared.snapshot.context.source_snapshot_id,
                                            prepared.snapshot.manifest.omitted_files.len() as u64,
                                            Some(&current_git_state.digest),
                                            prepared.local_git_state_digest.as_deref(),
                                        ) =>
                                {
                                    Freshness::Current
                                }
                                Ok(_) => Freshness::Stale,
                                Err(error) => {
                                    tracing::warn!(
                                        repository_id = %identity.repository_id(),
                                        error = %error,
                                        "local source changed or became unavailable after indexing; freshness is unknown"
                                    );
                                    Freshness::Unknown
                                }
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                repository_id = %identity.repository_id(),
                                error = %error,
                                "local Git state could not be revalidated after indexing"
                            );
                            Freshness::Unknown
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        repository_id = %identity.repository_id(),
                        error = %error,
                        "local repository could not be reopened after indexing"
                    );
                    Freshness::Unknown
                }
            };
            state.record_local_git_state_digest(prepared.local_git_state_digest.as_deref())?;
            return Ok(freshness);
        }

        // Remote repositories are checked against the exact commit captured by
        // the owned clone. A moved ref means this graph is valid for its pinned
        // commit but is not current for the requested remote.
        let observation = match identity.remote_locator() {
            Some(locator) => {
                git::observe_remote_freshness(
                    locator,
                    requested_ref,
                    prepared.git_repository.head_commit(),
                    &self.git_config,
                )
                .await
            }
            None => Err(git::GitError::InvalidRemoteLocator),
        };
        match observation {
            Ok(observation) => {
                let checked_at = observation
                    .remote_head
                    .as_ref()
                    .map(|head| head.checked_at)
                    .unwrap_or_else(chrono::Utc::now);
                let (remote_state, observed_head) =
                    match (observation.freshness, observation.remote_head.as_ref()) {
                        (git::RemoteFreshness::Unchanged, Some(head)) => (
                            RemoteFreshnessState::Unchanged,
                            Some(head.commit_id.as_str()),
                        ),
                        (git::RemoteFreshness::Changed, Some(head)) => {
                            (RemoteFreshnessState::Changed, Some(head.commit_id.as_str()))
                        }
                        _ => (RemoteFreshnessState::Unknown, None),
                    };
                state.record_remote_observation(observed_head, remote_state, checked_at)?;
                state.record_local_git_state_digest(None)?;
                Ok(match remote_state {
                    RemoteFreshnessState::Unchanged => Freshness::Current,
                    RemoteFreshnessState::Changed => Freshness::Stale,
                    RemoteFreshnessState::Unknown => Freshness::Unknown,
                })
            }
            Err(error) => {
                tracing::warn!(
                    repository_id = %identity.repository_id(),
                    error = %error,
                    "remote freshness could not be revalidated after indexing"
                );
                state.record_remote_observation(
                    None,
                    RemoteFreshnessState::Unknown,
                    chrono::Utc::now(),
                )?;
                state.record_local_git_state_digest(None)?;
                Ok(Freshness::Unknown)
            }
        }
    }

    /// Reconcile a durable cleanup-pending state when the operation directory
    /// is already absent. This covers a crash after the marker-last removal but
    /// before `Cleaned` could be persisted. An existing, symlinked, or unexpected
    /// path is never treated as successful cleanup.
    fn reconcile_missing_workspace_cleanup(
        &self,
        state: &mut RepositoryState,
    ) -> Result<(), RepositoryManagerError> {
        if !matches!(
            state.cleanup_state,
            CleanupState::CleanupPending | CleanupState::CleanupFailed
        ) {
            return Ok(());
        }

        let operation_id = state
            .last_workspace_operation_id
            .clone()
            .ok_or(RepositoryManagerError::StateUnavailable)?;
        let Some(_) = self.paths.existing_workspaces_dir()? else {
            return self.mark_workspace_cleanup_reconciled(state, &operation_id);
        };
        let workspace_path = self.paths.workspace_dir(&operation_id)?;
        match fs::symlink_metadata(&workspace_path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.mark_workspace_cleanup_reconciled(state, &operation_id)
            }
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(RepositoryManagerError::StateUnavailable)
            }
            Ok(_) => Err(RepositoryManagerError::StateUnavailable),
            Err(source) => Err(crate::repository::WorkspaceError::Io {
                path: workspace_path,
                source,
            }
            .into()),
        }
    }

    fn mark_workspace_cleanup_reconciled(
        &self,
        state: &mut RepositoryState,
        operation_id: &str,
    ) -> Result<(), RepositoryManagerError> {
        if state.cleanup_state == CleanupState::CleanupFailed {
            state.set_cleanup_state(CleanupState::CleanupPending, Some(operation_id))?;
        }
        if state.cleanup_state == CleanupState::CleanupPending {
            state.set_cleanup_state(CleanupState::Cleaned, Some(operation_id))?;
        }
        state.save(&self.paths)?;
        Ok(())
    }

    /// Prepare an owned temporary workspace, resuming only a single validated
    /// retry candidate for this exact repository/workspace scope.
    ///
    /// An active marker without a corresponding durable state owner is adopted
    /// only for a brand-new scope with exactly one candidate. Explicitly
    /// abandoned cleanup may be retried after marker phase/reason validation;
    /// ambiguous or unverified cleanup states fail closed.
    fn prepare_owned_workspace(
        &self,
        state: &mut RepositoryState,
        identity: &RepositoryIdentity,
        scope_key: &str,
        new_operation_id: &str,
        requested_ref: Option<&str>,
    ) -> Result<(String, WorkspaceRetryMode), RepositoryManagerError> {
        self.reconcile_missing_workspace_cleanup(state)?;
        let candidates = self
            .workspaces
            .scan_owned_workspaces()?
            .into_iter()
            .filter(|marker| {
                marker.repository_id == identity.repository_id()
                    && marker.workspace_scope_key == scope_key
                    && is_retry_candidate(marker)
            })
            .collect::<Vec<_>>();

        let state_operation_id = state.last_workspace_operation_id.clone();
        let selected = match state.cleanup_state {
            CleanupState::Active | CleanupState::RetryableFailure => {
                let operation_id = state_operation_id
                    .as_deref()
                    .ok_or(RepositoryManagerError::StateUnavailable)?;
                let matching = candidates
                    .iter()
                    .filter(|marker| marker.operation_id == operation_id)
                    .collect::<Vec<_>>();
                if matching.len() != 1 || candidates.len() != 1 {
                    return Err(RepositoryManagerError::StateUnavailable);
                }
                Some((*matching[0]).clone())
            }
            CleanupState::NotCreated if state_operation_id.is_none() && candidates.len() == 1 => {
                Some(candidates[0].clone())
            }
            CleanupState::NotCreated if candidates.is_empty() => None,
            CleanupState::NotCreated => {
                return Err(RepositoryManagerError::StateUnavailable);
            }
            CleanupState::Cleaned if candidates.is_empty() => None,
            CleanupState::Cleaned => {
                // A surviving unverified workspace next to a state record
                // that claims cleanup completed is contradictory. Do not
                // silently adopt or delete it.
                return Err(RepositoryManagerError::StateUnavailable);
            }
            CleanupState::CleanupPending | CleanupState::CleanupFailed => {
                let operation_id = state_operation_id
                    .as_deref()
                    .ok_or(RepositoryManagerError::StateUnavailable)?;
                let marker = self.workspaces.load_marker(operation_id)?;
                if !matches!(
                    marker.phase,
                    WorkspacePhase::CleanupPending | WorkspacePhase::CleanupFailed
                ) || marker.cleanup_reason
                    != Some(crate::repository::WorkspaceCleanupReason::RetryAbandoned)
                {
                    // GraphsVerified cleanup still requires a fresh graph proof;
                    // OperationFailed is retained for retry, not auto-deleted.
                    return Err(RepositoryManagerError::StateUnavailable);
                }
                self.workspaces.retry_abandoned_cleanup(operation_id)?;
                if state.cleanup_state == CleanupState::CleanupFailed {
                    state.set_cleanup_state(CleanupState::CleanupPending, Some(operation_id))?;
                }
                state.set_cleanup_state(CleanupState::Cleaned, Some(operation_id))?;
                state.save(&self.paths)?;
                None
            }
        };

        if let Some(marker) = selected {
            if marker.requested_ref.as_deref() != requested_ref {
                return Err(RepositoryManagerError::InvalidRequest(
                    "a retryable workspace exists for a different requested Git ref; recover or explicitly abandon that workspace before changing refs"
                        .to_owned(),
                ));
            }
            let workspace_operation_id = marker.operation_id.clone();
            // Re-read after acquiring the scope lock. A marker whose identity
            // or requested ref changed since the scan cannot authorize retry.
            let current_marker = self.workspaces.load_marker(&workspace_operation_id)?;
            if current_marker.repository_id != identity.repository_id()
                || current_marker.workspace_scope_key != scope_key
                || current_marker.remote_locator != identity.remote_locator().unwrap_or_default()
                || current_marker.requested_ref.as_deref() != requested_ref
                || current_marker.workspace_root != marker.workspace_root
                || current_marker.repository_root != marker.repository_root
            {
                return Err(RepositoryManagerError::StateUnavailable);
            }
            if current_marker.phase != WorkspacePhase::RetryableFailure {
                self.workspaces
                    .mark_retryable_failure(&workspace_operation_id, "operation_interrupted")?;
            }

            if state.cleanup_state != CleanupState::RetryableFailure {
                match state.cleanup_state {
                    CleanupState::NotCreated => {
                        state.set_cleanup_state(
                            CleanupState::Active,
                            Some(&workspace_operation_id),
                        )?;
                        state.set_cleanup_state(
                            CleanupState::RetryableFailure,
                            Some(&workspace_operation_id),
                        )?;
                    }
                    CleanupState::Active
                    | CleanupState::CleanupPending
                    | CleanupState::CleanupFailed => {
                        state.set_cleanup_state(
                            CleanupState::RetryableFailure,
                            Some(&workspace_operation_id),
                        )?;
                    }
                    CleanupState::RetryableFailure | CleanupState::Cleaned => {}
                }
            } else if state.last_workspace_operation_id.as_deref()
                != Some(workspace_operation_id.as_str())
            {
                return Err(RepositoryManagerError::StateUnavailable);
            }
            state.save(&self.paths)?;

            let retry_mode = self.workspaces.begin_retry(&workspace_operation_id)?;
            state.set_cleanup_state(CleanupState::Active, Some(&workspace_operation_id))?;
            state.save(&self.paths)?;
            return Ok((workspace_operation_id, retry_mode));
        }

        if !matches!(
            state.cleanup_state,
            CleanupState::NotCreated | CleanupState::Cleaned
        ) {
            return Err(RepositoryManagerError::StateUnavailable);
        }

        let marker =
            self.workspaces
                .create(new_operation_id, identity, scope_key, requested_ref)?;
        if let Err(error) =
            state.set_cleanup_state(CleanupState::Active, Some(&marker.operation_id))
        {
            let _ = self
                .workspaces
                .mark_retryable_failure(&marker.operation_id, "state_prepare_failed");
            return Err(error.into());
        }
        if let Err(error) = state.save(&self.paths) {
            let _ = self
                .workspaces
                .mark_retryable_failure(&marker.operation_id, "state_persist_failed");
            return Err(error.into());
        }

        Ok((marker.operation_id, WorkspaceRetryMode::CloneAgain))
    }

    /// Preserve a failed owned checkout for an explicit retry. This function
    /// never removes source files; deletion is reserved for verified cleanup
    /// or explicit abandonment through `WorkspaceRecovery`.
    fn record_retryable_workspace_failure(
        &self,
        state: &mut RepositoryState,
        operation_id: &str,
        failure_code: &str,
    ) {
        if let Err(error) = self
            .workspaces
            .mark_retryable_failure(operation_id, failure_code)
        {
            tracing::warn!(
                operation_id = %operation_id,
                error = %error,
                "could not mark owned workspace as retryable"
            );
        }

        if state.cleanup_state != CleanupState::RetryableFailure {
            let transition = match state.cleanup_state {
                CleanupState::NotCreated => state
                    .set_cleanup_state(CleanupState::Active, Some(operation_id))
                    .and_then(|_| {
                        state.set_cleanup_state(CleanupState::RetryableFailure, Some(operation_id))
                    }),
                CleanupState::Active
                | CleanupState::CleanupPending
                | CleanupState::CleanupFailed => {
                    state.set_cleanup_state(CleanupState::RetryableFailure, Some(operation_id))
                }
                CleanupState::RetryableFailure | CleanupState::Cleaned => Ok(()),
            };
            if let Err(error) = transition {
                tracing::warn!(
                    operation_id = %operation_id,
                    error = %error,
                    "could not persist retryable workspace state"
                );
            }
        }
        if let Err(error) = state.save(&self.paths) {
            tracing::warn!(
                operation_id = %operation_id,
                error = %error,
                "could not save retryable workspace state"
            );
        }
    }

    /// Repository-only resolution has no later ChIE stage to finish its
    /// operation. Mark that operation complete only after graph verification,
    /// durable state publication and any owned-workspace cleanup have finished.
    /// Investigation requests keep the operation running for `run_verified`.
    async fn complete_repository_only_operation(
        &self,
        operation: &mut Operation,
    ) -> Result<(), RepositoryManagerError> {
        operation.record.result_reference = Some(format!(
            "repository-knowledge:{}:{}",
            operation.record.context.repository_id, operation.record.context.source_snapshot_id,
        ));
        operation
            .transition(crate::contract::operation::OperationState::Completed)
            .map_err(RepositoryManagerError::Operation)?;
        let completed = operation
            .completed_event()
            .map_err(RepositoryManagerError::Operation)?;
        self.storage.persist_operation(&operation.record).await?;
        self.storage.append_operation_event(&completed).await?;
        Ok(())
    }

    async fn resolve_identity(
        &self,
        source_locator: &str,
    ) -> Result<(RepositoryIdentity, Option<PathBuf>), RepositoryManagerError> {
        if let Ok(identity) = RepositoryIdentity::from_remote(source_locator) {
            return Ok((identity, None));
        }
        let path = PathBuf::from(source_locator);
        if !path.is_absolute() {
            return Err(RepositoryManagerError::InvalidRequest(
                "local repository paths must be absolute".to_owned(),
            ));
        }
        let git_repository = git::open_local_repository(&path, &self.git_config).await?;
        let identity = RepositoryIdentity::from_local_directory_with_origin(
            &path,
            git_repository.remote_locator(),
        )?;
        Ok((identity, Some(path)))
    }

    async fn resolve_selected_entity(
        &self,
        context: &RepositoryContext,
        symbol_name: &str,
    ) -> Result<String, RepositoryManagerError> {
        let mut exact = self
            .storage
            .exact_code_entities_by_name(context, symbol_name)
            .await?;
        exact.sort_by(|left, right| left.id.cmp(&right.id));
        match exact.as_slice() {
            [entity] => Ok(entity.id.clone()),
            [] => Err(RepositoryManagerError::SelectedEntityNotFound),
            _ => Err(RepositoryManagerError::SelectedEntityAmbiguous),
        }
    }
}

fn is_retry_candidate(marker: &WorkspaceMarker) -> bool {
    matches!(
        marker.phase,
        WorkspacePhase::Preparing
            | WorkspacePhase::Cloning
            | WorkspacePhase::Analyzing
            | WorkspacePhase::Persisting
            | WorkspacePhase::Verifying
            | WorkspacePhase::RetryableFailure
    ) || (matches!(
        marker.phase,
        WorkspacePhase::CleanupPending | WorkspacePhase::CleanupFailed
    ) && marker.cleanup_reason
        == Some(crate::repository::WorkspaceCleanupReason::OperationFailed))
}

fn prepare_state(
    paths: &CodeMindPaths,
    repository_id: &str,
    workspace_id_for_state: &str,
    scope_key: &str,
) -> Result<RepositoryState, RepositoryManagerError> {
    let mut state = load_state_optional(paths, repository_id, scope_key)?.unwrap_or(
        RepositoryState::new_for_workspace(
            repository_id.to_owned(),
            workspace_id_for_state.to_owned(),
            scope_key.to_owned(),
        )?,
    );
    if state.analysis_state == AnalysisState::Ready {
        state.transition_analysis(AnalysisState::Stale)?;
    }
    if matches!(
        state.analysis_state,
        AnalysisState::Preparing
            | AnalysisState::Analyzing
            | AnalysisState::Persisting
            | AnalysisState::Verifying
    ) {
        // The caller holds the repository-scope lock. Any unfinished phase
        // that survived into this request belongs to a process that no longer
        // owns the lock and is therefore interrupted, not concurrently active.
        state.transition_analysis(AnalysisState::Failed)?;
    }
    if matches!(
        state.analysis_state,
        AnalysisState::Stale | AnalysisState::Failed | AnalysisState::Unknown
    ) {
        state.transition_analysis(AnalysisState::Preparing)?;
    }
    state.save(paths)?;
    Ok(state)
}

fn mark_failed_state(
    paths: &CodeMindPaths,
    state: &mut RepositoryState,
    operation_id: &str,
) -> Result<(), RepositoryManagerError> {
    if state.analysis_state != AnalysisState::Failed {
        let _ = state.transition_analysis(AnalysisState::Failed);
    }
    state.last_workspace_operation_id = Some(operation_id.to_owned());
    state.save(paths)?;
    Ok(())
}

fn load_state_optional(
    paths: &CodeMindPaths,
    repository_id: &str,
    scope_key: &str,
) -> Result<Option<RepositoryState>, RepositoryManagerError> {
    match RepositoryState::load(paths, repository_id, scope_key) {
        Ok(state) => Ok(Some(state)),
        Err(RepositoryStateError::Io { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        // Older state schemas cannot prove the current per-graph verification
        // contract and are rebuilt through the normal initialization path.
        // A newer schema is not an absent state: overwriting it could destroy
        // metadata written by a newer engine, so fail closed instead.
        Err(RepositoryStateError::UnsupportedSchemaVersion { actual, .. })
            if actual < crate::repository::REPOSITORY_STATE_SCHEMA_VERSION =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

fn context_from_state(
    state: &RepositoryState,
) -> Result<RepositoryContext, RepositoryManagerError> {
    Ok(RepositoryContext {
        repository_id: state.repository_id.clone(),
        workspace_id: state.workspace_id.clone(),
        source_snapshot_id: state
            .last_source_snapshot_id
            .clone()
            .ok_or(RepositoryManagerError::StateUnavailable)?,
        graph_version: state
            .graph_projection_version
            .clone()
            .ok_or(RepositoryManagerError::StateUnavailable)?,
    })
}

fn verification_expectation(
    state: &RepositoryState,
) -> crate::graph_storage::GraphVerificationExpectation {
    let verification = state
        .verification
        .as_ref()
        .expect("ready repository state must carry a verification receipt");
    crate::graph_storage::GraphVerificationExpectation {
        context: crate::contract::context::RepositoryContext {
            repository_id: verification.repository_id.clone(),
            workspace_id: verification.workspace_id.clone(),
            source_snapshot_id: verification.source_snapshot_id.clone(),
            graph_version: verification.graph_projection_version.clone(),
        },
        generation_id: verification.generation_id.clone(),
        code_entities: verification.code_entities as usize,
        code_relationships: verification.code_relationships as usize,
        code_entity_ids_digest: verification.code_entity_ids_digest.clone(),
        code_relationship_ids_digest: verification.code_relationship_ids_digest.clone(),
        test_entities: verification.test_entities as usize,
        test_relationships: verification.test_relationships as usize,
        test_entity_ids_digest: verification.test_entity_ids_digest.clone(),
        test_relationship_ids_digest: verification.test_relationship_ids_digest.clone(),
        history_entities: verification.history_entities as usize,
        history_relationships: verification.history_relationships as usize,
        history_head_commit: verification.history_head_commit.clone(),
        history_complete: verification.history_complete,
        history_omitted_commits_at_least: verification.history_omitted_commits_at_least,
        history_omitted_path_changes_at_least: verification.history_omitted_path_changes_at_least,
        change_entities: verification.change_entities as usize,
        change_relationships: verification.change_relationships as usize,
    }
}

fn local_snapshot_matches_verified_state(
    context_snapshot_id: &str,
    manifest_snapshot_id: &str,
    omitted_file_count: u64,
    expected_snapshot_id: &str,
    expected_omitted_file_count: u64,
    observed_git_state_digest: Option<&str>,
    expected_git_state_digest: Option<&str>,
) -> bool {
    context_snapshot_id == expected_snapshot_id
        && manifest_snapshot_id == expected_snapshot_id
        && omitted_file_count == expected_omitted_file_count
        && observed_git_state_digest.is_some()
        && observed_git_state_digest == expected_git_state_digest
}

#[allow(clippy::too_many_arguments)]
fn incremental_update_base_is_complete(
    base_is_complete: bool,
    omitted_file_count: u64,
    history_complete: bool,
    omitted_commits_at_least: u64,
    omitted_path_changes_at_least: u64,
    current_source_has_omissions: bool,
    current_history_complete: bool,
    current_history_omitted_commits_at_least: u64,
    current_history_omitted_path_changes_at_least: u64,
    current_local_worktree_clean: Option<bool>,
) -> bool {
    base_is_complete
        && omitted_file_count == 0
        && history_complete
        && omitted_commits_at_least == 0
        && omitted_path_changes_at_least == 0
        && !current_source_has_omissions
        && current_history_complete
        && current_history_omitted_commits_at_least == 0
        && current_history_omitted_path_changes_at_least == 0
        && current_local_worktree_clean != Some(false)
}

/// Aggregate the per-commit path omission lower bounds exposed by GitHistory.
/// GitHistory intentionally keeps path omission counts on each GitCommit,
/// because the omission can be caused by an individual commit's output bound.
/// An incomplete path list with a zero reported count still proves that at
/// least one path was omitted, so retain a conservative lower bound of one.
fn history_path_omissions_at_least(commit_omissions: impl IntoIterator<Item = (bool, u64)>) -> u64 {
    commit_omissions
        .into_iter()
        .fold(0_u64, |total, (complete, omitted)| {
            let lower_bound = if complete { omitted } else { omitted.max(1) };
            total.saturating_add(lower_bound)
        })
}

fn should_complete_repository_operation(symbol_name: Option<&str>) -> bool {
    symbol_name.is_none()
}

fn validate_request_component(
    kind: &'static str,
    value: &str,
) -> Result<(), RepositoryManagerError> {
    if value.trim().is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
        return Err(RepositoryManagerError::InvalidRequest(format!(
            "{kind} is empty, too long, or contains control characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        history_path_omissions_at_least, incremental_update_base_is_complete,
        local_snapshot_matches_verified_state,
    };

    #[test]
    fn history_path_omissions_are_aggregated_from_each_commit() {
        assert_eq!(
            history_path_omissions_at_least([(true, 0), (false, 2), (false, 1), (true, 0),]),
            3,
        );
    }

    #[test]
    fn incomplete_commit_path_list_has_nonzero_lower_bound() {
        assert_eq!(history_path_omissions_at_least([(false, 0)]), 1);
        assert_eq!(history_path_omissions_at_least([(true, 0)]), 0);
    }

    #[test]
    fn history_path_omission_aggregation_saturates_on_overflow() {
        assert_eq!(
            history_path_omissions_at_least([(false, u64::MAX), (false, 1)]),
            u64::MAX,
        );
    }

    #[test]
    fn local_snapshot_freshness_requires_exact_snapshot_and_omission_count() {
        let digest = "a".repeat(64);
        assert!(local_snapshot_matches_verified_state(
            "snapshot-a",
            "snapshot-a",
            0,
            "snapshot-a",
            0,
            Some(&digest),
            Some(&digest),
        ));
        assert!(!local_snapshot_matches_verified_state(
            "snapshot-a",
            "snapshot-a",
            0,
            "snapshot-b",
            0,
            Some(&digest),
            Some(&digest),
        ));
        assert!(!local_snapshot_matches_verified_state(
            "snapshot-a",
            "snapshot-a",
            1,
            "snapshot-a",
            0,
            Some(&digest),
            Some(&digest),
        ));
        assert!(!local_snapshot_matches_verified_state(
            "snapshot-a",
            "snapshot-b",
            0,
            "snapshot-a",
            0,
            Some(&digest),
            Some(&digest),
        ));
    }

    #[test]
    fn local_snapshot_reuse_requires_matching_git_state_fingerprint() {
        let digest = "a".repeat(64);
        let other = "b".repeat(64);
        assert!(local_snapshot_matches_verified_state(
            "snapshot-a",
            "snapshot-a",
            0,
            "snapshot-a",
            0,
            Some(&digest),
            Some(&digest),
        ));
        assert!(!local_snapshot_matches_verified_state(
            "snapshot-a",
            "snapshot-a",
            0,
            "snapshot-a",
            0,
            Some(&other),
            Some(&digest),
        ));
        assert!(!local_snapshot_matches_verified_state(
            "snapshot-a",
            "snapshot-a",
            0,
            "snapshot-a",
            0,
            Some(&digest),
            None,
        ));
    }

    #[test]
    fn incremental_updates_require_a_complete_base_and_current_source_capture() {
        assert!(incremental_update_base_is_complete(
            true, 0, true, 0, 0, false, true, 0, 0, None,
        ));
        assert!(!incremental_update_base_is_complete(
            false, 0, true, 0, 0, false, true, 0, 0, None,
        ));
        assert!(!incremental_update_base_is_complete(
            true, 1, true, 0, 0, false, true, 0, 0, None,
        ));
        assert!(!incremental_update_base_is_complete(
            true, 0, false, 0, 0, false, true, 0, 0, None,
        ));
        assert!(!incremental_update_base_is_complete(
            true, 0, true, 0, 0, true, true, 0, 0, None,
        ));
        assert!(!incremental_update_base_is_complete(
            true, 0, true, 0, 0, false, false, 0, 0, None,
        ));
        assert!(!incremental_update_base_is_complete(
            true, 0, true, 0, 0, false, true, 1, 0, None,
        ));
        assert!(!incremental_update_base_is_complete(
            true, 0, true, 0, 0, false, true, 0, 1, None,
        ));
        assert!(incremental_update_base_is_complete(
            true,
            0,
            true,
            0,
            0,
            false,
            true,
            0,
            0,
            Some(true),
        ));
        assert!(!incremental_update_base_is_complete(
            true,
            0,
            true,
            0,
            0,
            false,
            true,
            0,
            0,
            Some(false),
        ));
    }

    #[test]
    fn repository_resolution_completes_without_a_follow_up_investigation() {
        assert!(super::should_complete_repository_operation(None));
        assert!(!super::should_complete_repository_operation(Some(
            "symbol-1"
        )));
    }
}
