use crate::memory::{
    model::{EvidenceReference, HandoffRecord, MemoryRecord, ReviewContextRecord},
    scope::AccessContext,
    service,
};
use crate::{
    contract::generated::{
        self, ChangeAnalysisQuery, QueryResponse, SearchCodeQuery, SymbolRelationshipQuery,
        intelligence_service_server::IntelligenceService, query_response,
        repository_service_server::RepositoryService,
    },
    graph_storage::{GraphStorage, StorageError},
    repository::RepositoryManager,
    source::RepositoryContext,
};
use futures::Stream;
use std::{pin::Pin, sync::Arc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

// The domain contract distinguishes `Freshness::Unknown`, but the current
// protobuf enum does not expose an `Unknown` variant. Use its zero/default
// value for graph-only queries whose live repository freshness was not
// revalidated; the response limitation explains this state to clients.
const UNVERIFIED_FRESHNESS_PROTO_VALUE: i32 = 0;

/// Preserve the existing source-manifest JSON shape while exposing the
/// independently verified repository freshness/completeness metadata. Reused
/// repositories may have no source manifest because their workspace was
/// intentionally deleted; in that case the metadata object remains the payload.
fn repository_resolution_data_json(
    manifest_json: Option<&str>,
    knowledge_metadata: serde_json::Value,
) -> Result<String, serde_json::Error> {
    let Some(manifest_json) = manifest_json else {
        let mut metadata_payload = knowledge_metadata.as_object().cloned().unwrap_or_default();
        metadata_payload.insert("repository_knowledge".to_owned(), knowledge_metadata);
        return serde_json::to_string(&serde_json::Value::Object(metadata_payload));
    };
    let mut manifest: serde_json::Value = serde_json::from_str(manifest_json)?;
    if let Some(fields) = manifest.as_object_mut() {
        fields.insert("repository_knowledge".to_owned(), knowledge_metadata);
        serde_json::to_string(&manifest)
    } else {
        serde_json::to_string(&serde_json::json!({
            "manifest": manifest,
            "repository_knowledge": knowledge_metadata,
        }))
    }
}

pub struct IntelligenceServer<S> {
    storage: Arc<S>,
}

impl<S> IntelligenceServer<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }
}

pub struct MemoryServer<S> {
    storage: Arc<S>,
}

impl<S> MemoryServer<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }
}

pub struct OperationServer<S> {
    storage: Arc<S>,
    repository_manager: Arc<RepositoryManager<S>>,
}

impl<S> OperationServer<S> {
    /// Construct the production operation service. Repository preparation and
    /// source/indexing lifecycle are owned exclusively by the shared manager.
    pub fn new(storage: Arc<S>, repository_manager: Arc<RepositoryManager<S>>) -> Self {
        Self {
            storage,
            repository_manager,
        }
    }
}

#[tonic::async_trait]
impl<S> generated::operation_service_server::OperationService for OperationServer<S>
where
    S: GraphStorage + 'static,
{
    type StreamProgressStream =
        Pin<Box<dyn Stream<Item = Result<generated::ProgressEvent, Status>> + Send + 'static>>;

    async fn start_investigation(
        &self,
        request: Request<generated::StartInvestigationRequest>,
    ) -> Result<Response<generated::StartInvestigationResponse>, Status> {
        let input = request.into_inner();
        let context = repository_context(
            input
                .context
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        if input.source_path.trim().is_empty() {
            return Err(Status::invalid_argument("source_path is required"));
        }
        if input.symbol_name.trim().is_empty() {
            return Err(Status::invalid_argument("symbol_name is required"));
        }

        let operation_id = uuid::Uuid::new_v4().to_string();
        let request_id = input
            .correlation
            .as_ref()
            .map(|correlation| correlation.request_id.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let trace_id = input
            .correlation
            .as_ref()
            .map(|correlation| correlation.trace_id.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let knowledge = self
            .repository_manager
            .ensure_indexed(
                &input.source_path,
                &context.workspace_id,
                None,
                &input.symbol_name,
                &operation_id,
                &request_id,
                &trace_id,
            )
            .await
            .map_err(repository_manager_status)?;

        let pipeline =
            crate::engines::pipeline::InvestigationPipeline::new(Arc::clone(&self.storage));
        let verified_request = crate::engines::pipeline::VerifiedPipelineRequest {
            context: knowledge.context.clone(),
            selected_entity_id: knowledge.selected_entity_id.clone(),
            request_id,
            trace_id,
            operation_id: operation_id.clone(),
            knowledge_completeness: knowledge.completeness,
            freshness: knowledge.freshness,
            omitted_file_count: knowledge.omitted_file_count,
            history_complete: knowledge.history_complete,
            history_omitted_commits_at_least: knowledge.history_omitted_commits_at_least,
            history_omitted_path_changes_at_least: knowledge.history_omitted_path_changes_at_least,
        };
        let pipeline_operation_id = operation_id.clone();
        tokio::spawn(async move {
            if let Err(error) = pipeline.run_verified(verified_request).await {
                tracing::error!(
                    %pipeline_operation_id,
                    %error,
                    "CodeMind verified investigation pipeline failed"
                );
            }
        });

        Ok(Response::new(generated::StartInvestigationResponse {
            operation_id,
            context: Some(crate::transport::conversions::context_proto(
                &knowledge.context,
            )),
            selected_entity_id: knowledge.selected_entity_id,
        }))
    }

    async fn stream_progress(
        &self,
        request: Request<generated::StreamProgressRequest>,
    ) -> Result<Response<Self::StreamProgressStream>, Status> {
        let input = request.into_inner();
        let context = repository_context(
            input
                .context
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        if input.operation_id.trim().is_empty() {
            return Err(Status::invalid_argument("operation_id is required"));
        }
        let limit = if input.limit == 0 {
            100usize
        } else {
            (input.limit as usize).min(1_000)
        };
        let storage = Arc::clone(&self.storage);
        let operation_id = input.operation_id;
        let mut after_sequence = input.after_sequence;
        let (sender, receiver) =
            tokio::sync::mpsc::channel::<Result<generated::ProgressEvent, Status>>(32);

        tokio::spawn(async move {
            loop {
                let operation = match storage.retrieve_operation(&context, &operation_id).await {
                    Ok(operation) => operation,
                    Err(error) => {
                        let _ = sender.send(Err(storage_status(error))).await;
                        break;
                    }
                };

                let events = match storage
                    .events_since(&context, &operation_id, after_sequence, limit)
                    .await
                {
                    Ok(events) => events,
                    Err(error) => {
                        let _ = sender.send(Err(storage_status(error))).await;
                        break;
                    }
                };

                let mut sent = false;
                for event in events {
                    after_sequence = after_sequence.max(event.metadata().sequence);
                    let progress = match event {
                        crate::contract::events::EngineEvent::Progress { .. } => {
                            let operation_event =
                                crate::lifecycle::operation::OperationEvent::from_engine_event(
                                    &event,
                                );
                            let value =
                                crate::lifecycle::progress::Progress::from_event(&operation_event);
                            crate::transport::progress::to_proto(&value)
                        }
                        _ => continue,
                    };
                    sent = true;
                    if sender.send(Ok(progress)).await.is_err() {
                        return;
                    }
                }

                if operation.state.is_terminal() && !sent {
                    break;
                }
                if !sent {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(receiver))))
    }
}

/// Repository gRPC boundary backed by the same lifecycle manager used by
/// investigations. Repository resolution is never a separate source-capture
/// path, so it cannot create repository state that the operation service does
/// not know about.
pub struct RepositoryServer<S> {
    repository_manager: Arc<RepositoryManager<S>>,
}

impl<S> RepositoryServer<S> {
    pub fn new(repository_manager: Arc<RepositoryManager<S>>) -> Self {
        Self { repository_manager }
    }
}

#[tonic::async_trait]
impl<S> RepositoryService for RepositoryServer<S>
where
    S: GraphStorage + 'static,
{
    async fn resolve_repository(
        &self,
        request: Request<generated::ResolveRepositoryRequest>,
    ) -> Result<Response<generated::ResolveRepositoryResponse>, Status> {
        let input = request.into_inner();
        let context = input
            .context
            .ok_or_else(|| Status::invalid_argument("context is required"))?;
        let workspace_id = crate::transport::conversions::workspace_id(&context.workspace_id)
            .map_err(Status::invalid_argument)?;
        if input.source_path.trim().is_empty() {
            return Err(Status::invalid_argument("source_path is required"));
        }

        let operation_id = uuid::Uuid::new_v4().to_string();
        let request_id = input
            .correlation
            .as_ref()
            .map(|correlation| correlation.request_id.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let trace_id = input
            .correlation
            .as_ref()
            .map(|correlation| correlation.trace_id.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Repository resolution has no selected symbol. Use a repository-level
        // readiness helper so it shares the exact same Rust-owned lifecycle as
        // investigations rather than reconstructing source state independently.
        let knowledge = self
            .repository_manager
            .ensure_repository_ready(
                &input.source_path,
                &workspace_id,
                None,
                &operation_id,
                &request_id,
                &trace_id,
            )
            .await
            .map_err(repository_manager_status)?;

        let knowledge_metadata = serde_json::json!({
            "repository_id": knowledge.context.repository_id,
            "source_snapshot_id": knowledge.context.source_snapshot_id,
            "graph_version": knowledge.context.graph_version,
            "complete": knowledge.complete,
            "completeness": knowledge.completeness,
            "freshness": knowledge.freshness,
            "omitted_file_count": knowledge.omitted_file_count,
            "history_complete": knowledge.history_complete,
            "history_omitted_commits_at_least": knowledge.history_omitted_commits_at_least,
            "history_omitted_path_changes_at_least": knowledge.history_omitted_path_changes_at_least,
            "cleanup_state": knowledge.cleanup_state,
            "head_commit": knowledge.head_commit,
            "reused": knowledge.reused,
        });
        let manifest_json =
            repository_resolution_data_json(knowledge.manifest_json.as_deref(), knowledge_metadata)
                .map_err(|error| {
                    Status::internal(format!("failed to serialize repository knowledge: {error}"))
                })?;

        Ok(Response::new(generated::ResolveRepositoryResponse {
            result: Some(generated::resolve_repository_response::Result::Snapshot(
                generated::RepositorySnapshot {
                    context: Some(crate::transport::conversions::context_proto(
                        &knowledge.context,
                    )),
                    repository_name: input
                        .source_path
                        .rsplit(['/', '\\'])
                        .next()
                        .filter(|name| !name.is_empty())
                        .unwrap_or("repository")
                        .to_owned(),
                    manifest_json,
                },
            )),
        }))
    }
}

#[tonic::async_trait]
impl<S> generated::memory_service_server::MemoryService for MemoryServer<S>
where
    S: crate::memory::repository::MemoryRepository + 'static,
{
    async fn persist_memory(
        &self,
        request: Request<generated::PersistMemoryRequest>,
    ) -> Result<Response<generated::MemoryResponse>, Status> {
        let principal = principal_id(&request)?;
        let input = request
            .into_inner()
            .memory
            .ok_or_else(|| Status::invalid_argument("memory is required"))?;
        let record = memory_record(input)?;
        let access = AccessContext::new(
            principal,
            record.scope.clone(),
            record.context.workspace_id.clone(),
        )
        .map_err(Status::invalid_argument)?;
        let saved = service::persist_memory_authorized(self.storage.as_ref(), &access, record)
            .await
            .map_err(storage_status)?;
        Ok(Response::new(generated::MemoryResponse {
            result: Some(generated::memory_response::Result::Memory(memory_proto(
                saved,
            ))),
        }))
    }

    async fn retrieve_memory(
        &self,
        request: Request<generated::RetrieveMemoryRequest>,
    ) -> Result<Response<generated::RetrieveMemoryResponse>, Status> {
        let input_ref = request.get_ref();
        let context = repository_context(
            input_ref
                .context
                .clone()
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        let scope = input_ref.scope.clone();
        let owner_id = input_ref.owner_id.clone();
        let access = access_context_from_context(&request, &context, &scope, &owner_id)?;
        let input = request.into_inner();
        let records = service::retrieve_memory_authorized(
            self.storage.as_ref(),
            &access,
            &context,
            &input.scope,
            &input.kind,
            &input.owner_id,
            input.limit as usize,
        )
        .await
        .map_err(storage_status)?;
        Ok(Response::new(generated::RetrieveMemoryResponse {
            memories: records.into_iter().map(memory_proto).collect(),
        }))
    }

    async fn persist_handoff(
        &self,
        request: Request<generated::PersistHandoffRequest>,
    ) -> Result<Response<generated::MemoryResponse>, Status> {
        let input = request
            .into_inner()
            .handoff
            .ok_or_else(|| Status::invalid_argument("handoff is required"))?;
        let saved = service::persist_handoff(self.storage.as_ref(), handoff_record(input)?)
            .await
            .map_err(storage_status)?;
        Ok(Response::new(generated::MemoryResponse {
            result: Some(generated::memory_response::Result::Handoff(handoff_proto(
                saved,
            )?)),
        }))
    }

    async fn retrieve_handoff(
        &self,
        request: Request<generated::RetrieveHandoffRequest>,
    ) -> Result<Response<generated::MemoryResponse>, Status> {
        let input = request.into_inner();
        let context = repository_context(
            input
                .context
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        let handoff = self
            .storage
            .retrieve_handoff(&context, &input.handoff_id)
            .await
            .map_err(storage_status)?;
        Ok(Response::new(generated::MemoryResponse {
            result: Some(generated::memory_response::Result::Handoff(handoff_proto(
                handoff,
            )?)),
        }))
    }

    async fn transition_handoff(
        &self,
        request: Request<generated::TransitionHandoffRequest>,
    ) -> Result<Response<generated::MemoryResponse>, Status> {
        let input = request.into_inner();
        let context = repository_context(
            input
                .context
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        let handoff = self
            .storage
            .transition_handoff(&context, &input.handoff_id, &input.status)
            .await
            .map_err(storage_status)?;
        Ok(Response::new(generated::MemoryResponse {
            result: Some(generated::memory_response::Result::Handoff(handoff_proto(
                handoff,
            )?)),
        }))
    }

    async fn persist_review_context(
        &self,
        request: Request<generated::PersistReviewContextRequest>,
    ) -> Result<Response<generated::MemoryResponse>, Status> {
        let principal = principal_id(&request)?;
        let input = request
            .into_inner()
            .review
            .ok_or_else(|| Status::invalid_argument("review is required"))?;
        let context = repository_context(
            input
                .context
                .clone()
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        let mut record =
            serde_json::from_str::<ReviewContextRecord>(&input.payload_json).map_err(|error| {
                Status::invalid_argument(format!("invalid review payload: {error}"))
            })?;
        if record.context_id != input.context_id || record.scope != input.scope {
            return Err(Status::invalid_argument(
                "review envelope does not match payload",
            ));
        }
        if record.context != context {
            return Err(Status::invalid_argument(
                "review context does not match envelope",
            ));
        }
        if record.scope == "user" || record.scope == "team" {
            if !record.owner_id.is_empty() && record.owner_id != principal {
                return Err(Status::permission_denied(
                    "review owner does not match caller principal",
                ));
            }
            record.owner_id = principal.clone();
        }
        let access = AccessContext::new(
            principal,
            record.scope.clone(),
            record.context.workspace_id.clone(),
        )
        .map_err(Status::invalid_argument)?;
        let saved =
            service::persist_review_context_authorized(self.storage.as_ref(), &access, record)
                .await
                .map_err(storage_status)?;
        Ok(Response::new(generated::MemoryResponse {
            result: Some(generated::memory_response::Result::Review(review_proto(
                saved,
            )?)),
        }))
    }

    async fn retrieve_review_context(
        &self,
        request: Request<generated::RetrieveReviewContextRequest>,
    ) -> Result<Response<generated::MemoryResponse>, Status> {
        let principal = principal_id(&request)?;
        let input = request.into_inner();
        let context = repository_context(
            input
                .context
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        let record = self
            .storage
            .retrieve_review_context(&context, &input.context_id)
            .await
            .map_err(storage_status)?;
        let access = AccessContext::new(
            principal,
            record.scope.clone(),
            record.context.workspace_id.clone(),
        )
        .map_err(Status::invalid_argument)?;
        if !access.allows_record(&record.scope, &record.owner_id, &record.context) {
            return Err(Status::permission_denied(
                "review scope authorization denied",
            ));
        }
        Ok(Response::new(generated::MemoryResponse {
            result: Some(generated::memory_response::Result::Review(review_proto(
                record,
            )?)),
        }))
    }
}

fn review_proto(record: ReviewContextRecord) -> Result<generated::ReviewContextRecord, Status> {
    let payload_json =
        serde_json::to_string(&record).map_err(|error| Status::internal(error.to_string()))?;
    Ok(generated::ReviewContextRecord {
        context_id: record.context_id,
        scope: record.scope,
        payload_json,
        context: Some(generated::RepositoryContext {
            repository_id: record.context.repository_id,
            workspace_id: record.context.workspace_id,
            source_snapshot_id: record.context.source_snapshot_id,
            graph_version: record.context.graph_version,
        }),
        updated_at: record.updated_at,
    })
}

fn repository_context(input: generated::RepositoryContext) -> Result<RepositoryContext, Status> {
    crate::transport::conversions::context(&input).map_err(Status::invalid_argument)
}

fn pipeline_status(error: crate::engines::pipeline::PipelineError) -> Status {
    use crate::engines::pipeline::PipelineError;
    match error {
        PipelineError::InvalidInput(message) => Status::invalid_argument(message),
        PipelineError::SelectedEntityNotFound => {
            Status::not_found("selected symbol or file was not found")
        }
        PipelineError::SelectedEntityAmbiguous => {
            Status::failed_precondition("selected symbol or file is ambiguous")
        }
        PipelineError::Storage(error) => storage_status(error),
        PipelineError::Engine(message) => Status::failed_precondition(message),
    }
}

fn repository_manager_status(error: crate::repository::RepositoryManagerError) -> Status {
    use crate::repository::{CodeMindPathsError, RepositoryManagerError, WorkspaceRecoveryError};
    match error {
        RepositoryManagerError::InvalidRequest(message) => Status::invalid_argument(message),
        RepositoryManagerError::SelectedEntityNotFound => {
            Status::not_found("selected symbol or file was not found")
        }
        RepositoryManagerError::SelectedEntityAmbiguous => {
            Status::failed_precondition("selected symbol or file is ambiguous")
        }
        RepositoryManagerError::Storage(error) => storage_status(error),
        RepositoryManagerError::Operation(message) => Status::internal(message),
        RepositoryManagerError::Identity(error) => Status::invalid_argument(error.to_string()),
        RepositoryManagerError::State(error) => Status::failed_precondition(error.to_string()),
        RepositoryManagerError::Workspace(error) => Status::failed_precondition(error.to_string()),
        RepositoryManagerError::Git(error) => Status::failed_precondition(error.to_string()),
        RepositoryManagerError::Source(error) => Status::failed_precondition(error.to_string()),
        RepositoryManagerError::Indexing(error) => Status::failed_precondition(error.to_string()),
        RepositoryManagerError::StateUnavailable => {
            Status::failed_precondition("repository state is unavailable")
        }
        RepositoryManagerError::Paths(error) => {
            let message = error.to_string();
            match error {
                CodeMindPathsError::InvalidPathComponent { .. } => {
                    Status::invalid_argument(message)
                }
                _ => Status::internal(message),
            }
        }
        RepositoryManagerError::Recovery(error) => {
            let message = error.to_string();
            match error {
                WorkspaceRecoveryError::ScopeBusy => Status::aborted(message),
                WorkspaceRecoveryError::Workspace(_) | WorkspaceRecoveryError::State(_) => {
                    Status::failed_precondition(message)
                }
                WorkspaceRecoveryError::Paths(_) => Status::internal(message),
            }
        }
    }
}

fn evidence(input: Vec<generated::SourceReference>) -> Vec<EvidenceReference> {
    input
        .into_iter()
        .map(|reference| EvidenceReference {
            path: reference.path,
            start_line: reference.start_line,
            end_line: reference.end_line,
            snapshot_id: reference.snapshot_id,
        })
        .collect()
}

fn memory_record(input: generated::MemoryRecord) -> Result<MemoryRecord, Status> {
    let context = repository_context(
        input
            .context
            .ok_or_else(|| Status::invalid_argument("context is required"))?,
    )?;
    Ok(MemoryRecord {
        memory_id: input.memory_id,
        owner_id: input.owner_id,
        kind: input.kind,
        scope: input.scope,
        lifecycle: input.lifecycle,
        freshness: input.freshness,
        payload_json: input.payload_json,
        context,
        evidence: evidence(input.evidence),
        idempotency_key: input.idempotency_key,
        version: input.version,
        created_at: input.created_at,
        updated_at: input.updated_at,
    })
}

fn handoff_record(input: generated::Handoff) -> Result<HandoffRecord, Status> {
    let context = repository_context(
        input
            .context
            .ok_or_else(|| Status::invalid_argument("context is required"))?,
    )?;
    serde_json::from_str(&input.payload_json)
        .map_err(|error| Status::invalid_argument(format!("invalid handoff payload: {error}")))
        .and_then(|record: HandoffRecord| {
            if record.handoff_id != input.handoff_id
                || record.status != input.status
                || record.context != context
            {
                return Err(Status::invalid_argument(
                    "handoff envelope does not match payload",
                ));
            }
            Ok(record)
        })
}

fn memory_proto(record: MemoryRecord) -> generated::MemoryRecord {
    generated::MemoryRecord {
        memory_id: record.memory_id,
        owner_id: record.owner_id,
        kind: record.kind,
        scope: record.scope,
        lifecycle: record.lifecycle,
        freshness: record.freshness,
        payload_json: record.payload_json,
        context: Some(generated::RepositoryContext {
            repository_id: record.context.repository_id,
            workspace_id: record.context.workspace_id,
            source_snapshot_id: record.context.source_snapshot_id,
            graph_version: record.context.graph_version,
        }),
        evidence: record
            .evidence
            .into_iter()
            .map(|reference| generated::SourceReference {
                path: reference.path,
                start_line: reference.start_line,
                start_column: 0,
                end_line: reference.end_line,
                end_column: 0,
                snapshot_id: reference.snapshot_id,
            })
            .collect(),
        idempotency_key: record.idempotency_key,
        version: record.version,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn handoff_proto(record: HandoffRecord) -> Result<generated::Handoff, Status> {
    let payload_json = serde_json::to_string(&record)
        .map_err(|error| Status::internal(format!("failed to serialize handoff: {error}")))?;
    Ok(generated::Handoff {
        handoff_id: record.handoff_id,
        status: record.status,
        payload_json,
        context: Some(generated::RepositoryContext {
            repository_id: record.context.repository_id,
            workspace_id: record.context.workspace_id,
            source_snapshot_id: record.context.source_snapshot_id,
            graph_version: record.context.graph_version,
        }),
    })
}

#[tonic::async_trait]
impl<S> IntelligenceService for IntelligenceServer<S>
where
    S: GraphStorage + 'static,
{
    async fn search_code(
        &self,
        request: Request<SearchCodeQuery>,
    ) -> Result<Response<QueryResponse>, Status> {
        let input = request.into_inner();
        let search_request = crate::transport::conversions::search_code_request(input)
            .map_err(Status::invalid_argument)?;
        let context = search_request.context.clone();
        let query = search_request.query.clone();
        let pipeline =
            crate::engines::pipeline::InvestigationPipeline::new(Arc::clone(&self.storage));
        let search = pipeline
            .search_code(search_request)
            .await
            .map_err(pipeline_status)?;
        let matches = search.matches;
        let truncated = search.truncated;
        let data = serde_json::json!({
            "query": query,
            "matches": &matches,
            "truncated": truncated,
        });
        let evidence = matches
            .iter()
            .filter(|entity| !entity.path.is_empty() && entity.line > 0)
            .map(|entity| generated::SourceReference {
                path: entity.path.clone(),
                start_line: entity.line,
                start_column: 0,
                end_line: entity.line,
                end_column: 0,
                snapshot_id: context.source_snapshot_id.clone(),
            })
            .collect();

        Ok(Response::new(QueryResponse {
            result: Some(query_response::Result::Data(generated::CanonicalResult {
                outcome: if truncated {
                    generated::Outcome::Partial as i32
                } else if matches.is_empty() {
                    generated::Outcome::Empty as i32
                } else {
                    generated::Outcome::Success as i32
                },
                completeness: if truncated {
                    generated::Completeness::Truncated as i32
                } else {
                    generated::Completeness::Complete as i32
                },
                freshness: UNVERIFIED_FRESHNESS_PROTO_VALUE,
                context: Some(crate::transport::conversions::context_proto(&context)),
                correlation: None,
                data_json: serde_json::to_string(&data)
                    .map_err(|error| Status::internal(error.to_string()))?,
                evidence,
                diagnostics: Vec::new(),
                limitations: {
                    let mut limitations = vec![
                        "Repository freshness was not revalidated for this graph query; results are scoped to the supplied snapshot.".to_owned(),
                    ];
                    if truncated {
                        limitations.push(
                            "search results were truncated at the requested bound".to_owned(),
                        );
                    }
                    limitations
                },
            })),
        }))
    }

    async fn query_symbol_relationships(
        &self,
        request: Request<SymbolRelationshipQuery>,
    ) -> Result<Response<QueryResponse>, Status> {
        let input = request.into_inner();
        let relationship_request =
            crate::transport::conversions::symbol_relationship_request(input)
                .map_err(Status::invalid_argument)?;
        let context = relationship_request.context.clone();
        let pipeline =
            crate::engines::pipeline::InvestigationPipeline::new(Arc::clone(&self.storage));
        let query = pipeline
            .query_symbol_relationships(relationship_request)
            .await
            .map_err(pipeline_status)?;
        let relationships = query.relationships;
        let truncated = query.truncated;

        let data = serde_json::to_string(&relationships)
            .map_err(|error| Status::internal(error.to_string()))?;

        Ok(Response::new(QueryResponse {
            result: Some(query_response::Result::Data(generated::CanonicalResult {
                outcome: if truncated {
                    generated::Outcome::Partial as i32
                } else if relationships.is_empty() {
                    generated::Outcome::Empty as i32
                } else {
                    generated::Outcome::Success as i32
                },
                completeness: if truncated {
                    generated::Completeness::Truncated as i32
                } else {
                    generated::Completeness::Complete as i32
                },
                freshness: UNVERIFIED_FRESHNESS_PROTO_VALUE,
                context: Some(crate::transport::conversions::context_proto(&context)),
                correlation: None,
                data_json: data,
                evidence: relationships
                    .iter()
                    .filter_map(|relationship| relationship.evidence.as_ref())
                    .map(|evidence| generated::SourceReference {
                        path: evidence.path.clone(),
                        start_line: evidence.start_line,
                        start_column: 0,
                        end_line: evidence.end_line,
                        end_column: 0,
                        snapshot_id: evidence.snapshot_id.clone(),
                    })
                    .collect(),
                diagnostics: Vec::new(),
                limitations: {
                    let mut limitations = vec![
                        "Repository freshness was not revalidated for this graph query; results are scoped to the supplied snapshot.".to_owned(),
                    ];
                    if truncated {
                        limitations.push(
                            "relationship result was truncated at the requested bound".to_owned(),
                        );
                    }
                    limitations
                },
            })),
        }))
    }

    async fn analyze_change(
        &self,
        request: Request<ChangeAnalysisQuery>,
    ) -> Result<Response<QueryResponse>, Status> {
        let input = request.into_inner();
        let analysis_request = crate::transport::conversions::change_analysis_request(input)
            .map_err(Status::invalid_argument)?;
        tracing::info!(
            repository_id = %analysis_request.context.repository_id,
            source_snapshot_id = %analysis_request.context.source_snapshot_id,
            symbol_id = %analysis_request.target_entity_id,
            max_depth = analysis_request.max_depth,
            "Rust engine started change analysis query"
        );

        let pipeline =
            crate::engines::pipeline::InvestigationPipeline::new(Arc::clone(&self.storage));
        let mut analysis = pipeline
            .analyze_change_query(analysis_request)
            .await
            .map_err(pipeline_status)?;

        tracing::info!(
            repository_id = %analysis.context.repository_id,
            source_snapshot_id = %analysis.context.source_snapshot_id,
            symbol_id = %analysis.selected_entity_id,
            impact_paths = analysis.impact_paths.len(),
            test_candidates = analysis.test_candidates.len(),
            truncated = analysis.truncated,
            "Rust engine completed change analysis query"
        );

        analysis.limitations.push(
            "Repository freshness was not revalidated for this graph query; analysis is scoped to the supplied snapshot.".to_owned(),
        );
        let analysis = fit_change_analysis_result(analysis);
        let data_json = serde_json::to_string(&analysis)
            .map_err(|error| Status::internal(error.to_string()))?;

        let evidence = analysis
            .evidence_records
            .iter()
            .filter_map(|record| record.source.as_ref())
            .map(|source| generated::SourceReference {
                path: source.path.clone(),
                start_line: source.start_line,
                start_column: 0,
                end_line: source.end_line,
                end_column: 0,
                snapshot_id: source.snapshot_id.clone(),
            })
            .collect();

        Ok(Response::new(QueryResponse {
            result: Some(query_response::Result::Data(generated::CanonicalResult {
                outcome: if analysis.truncated {
                    generated::Outcome::Partial as i32
                } else if analysis.impact_paths.is_empty() {
                    generated::Outcome::Empty as i32
                } else {
                    generated::Outcome::Success as i32
                },
                completeness: if analysis.truncated {
                    generated::Completeness::Truncated as i32
                } else {
                    generated::Completeness::Complete as i32
                },
                freshness: UNVERIFIED_FRESHNESS_PROTO_VALUE,
                context: Some(crate::transport::conversions::context_proto(
                    &analysis.context,
                )),
                correlation: None,
                data_json,
                evidence,
                diagnostics: Vec::new(),
                limitations: analysis.limitations,
            })),
        }))
    }
}

/// Keep the complete canonical envelope below the shared 256 KiB result
/// contract. ChIE's nested path and evidence records can otherwise exceed the
/// byte bound even when each individual list respects its item-count limit.
fn fit_change_analysis_result(
    mut analysis: crate::engines::chie::model::ChangeAnalysis,
) -> crate::engines::chie::model::ChangeAnalysis {
    use crate::contract::result::MAX_SERIALIZED_BYTES;
    const DATA_BUDGET: usize = MAX_SERIALIZED_BYTES - 64 * 1024;

    let serialized_len = |value: &crate::engines::chie::model::ChangeAnalysis| {
        serde_json::to_vec(value)
            .map(|bytes| bytes.len())
            .unwrap_or(usize::MAX)
    };
    while serialized_len(&analysis) > DATA_BUDGET {
        let removed = if !analysis.impact_paths.is_empty() {
            analysis.impact_paths.pop();
            if let Some(impact) = analysis.impact.as_mut() {
                impact.paths.pop();
                impact.impacted_entity_ids = std::iter::once(analysis.selected_entity_id.clone())
                    .chain(impact.paths.iter().map(|path| path.entity_id.clone()))
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                impact.truncated = true;
                impact.availability = crate::engines::chie::model::EvidenceAvailability::Partial;
            }
            true
        } else if !analysis.test_candidates.is_empty() {
            analysis.test_candidates.pop();
            true
        } else if !analysis.evidence_records.is_empty() {
            analysis.evidence_records.pop();
            analysis.evidence = analysis
                .evidence_records
                .iter()
                .filter_map(|record| record.relationship_id.clone())
                .collect();
            if let Some(impact) = analysis.impact.as_mut() {
                impact.evidence = analysis
                    .evidence_records
                    .iter()
                    .filter_map(|record| record.source.clone())
                    .collect();
            }
            true
        } else if analysis.evidence.pop().is_some() {
            true
        } else if let Some(impact) = analysis
            .impact
            .as_mut()
            .filter(|impact| !impact.evidence.is_empty())
        {
            impact.evidence.pop();
            true
        } else {
            false
        };
        if !removed {
            break;
        }
        analysis.truncated = true;
        if !analysis
            .limitations
            .iter()
            .any(|item| item.contains("serialized result byte limit"))
        {
            analysis.limitations.push(
                "Evidence was bounded to fit the canonical serialized result byte limit."
                    .to_owned(),
            );
        }
    }
    analysis
}

fn principal_id<T>(request: &Request<T>) -> Result<String, Status> {
    let value = request
        .metadata()
        .get("x-codemind-principal")
        .ok_or_else(|| Status::unauthenticated("x-codemind-principal metadata is required"))?
        .to_str()
        .map_err(|_| {
            Status::invalid_argument("x-codemind-principal metadata must be valid ASCII")
        })?;
    if value.trim().is_empty() || value.len() > 512 {
        return Err(Status::invalid_argument(
            "x-codemind-principal is empty or too long",
        ));
    }
    Ok(value.to_owned())
}

fn access_context_from_context<T>(
    request: &Request<T>,
    context: &RepositoryContext,
    scope: &str,
    owner_id: &str,
) -> Result<AccessContext, Status> {
    let principal = principal_id(request)?;
    let access = AccessContext::new(principal, scope.to_owned(), context.workspace_id.clone())
        .map_err(Status::invalid_argument)?;
    if !access.allows_record(scope, owner_id, context) {
        return Err(Status::permission_denied(
            "memory scope authorization denied",
        ));
    }
    Ok(access)
}

fn schema_incompatible_status(expected: u32, actual: u32) -> Status {
    Status::failed_precondition(format!(
        "incompatible graph storage schema: expected {expected}, found {actual}"
    ))
}

fn storage_status(error: StorageError) -> Status {
    match error {
        StorageError::InvalidContext => Status::invalid_argument("invalid repository context"),
        StorageError::Unavailable(message) => Status::unavailable(message),
        StorageError::SchemaIncompatible { expected, actual } => {
            schema_incompatible_status(expected, actual)
        }
        StorageError::Mapping(message) => Status::internal(message),
        StorageError::Query(message) if message.contains("not found") => Status::not_found(message),
        StorageError::Query(message)
            if message.contains("conflict") || message.contains("idempotency") =>
        {
            Status::aborted(message)
        }
        StorageError::Query(message)
            if message.contains("invalid") || message.contains("unsupported") =>
        {
            Status::invalid_argument(message)
        }
        StorageError::Query(message) => Status::internal(message),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn schema_incompatibility_maps_to_failed_precondition() {
        let status = super::schema_incompatible_status(2, 1);

        assert_eq!(status.code(), tonic::Code::FailedPrecondition);
        assert!(status.message().contains("expected 2, found 1"));
    }

    #[test]
    fn repository_metadata_is_added_without_discarding_manifest_fields() {
        let metadata = serde_json::json!({
            "freshness": "unknown",
            "completeness": "partial",
            "omitted_file_count": 2,
        });
        let raw_manifest = r#"{"source_snapshot_id":"snapshot-a","file_count":3}"#;
        let encoded = super::repository_resolution_data_json(Some(raw_manifest), metadata)
            .expect("serialize merged repository data");
        let decoded: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded["source_snapshot_id"], "snapshot-a");
        assert_eq!(decoded["file_count"], 3);
        assert_eq!(decoded["repository_knowledge"]["freshness"], "unknown");
        assert_eq!(decoded["repository_knowledge"]["completeness"], "partial");
    }

    #[test]
    fn reused_repository_without_manifest_still_returns_knowledge_metadata() {
        let metadata = serde_json::json!({
            "freshness": "current",
            "completeness": "complete",
        });
        let encoded = super::repository_resolution_data_json(None, metadata)
            .expect("serialize reused repository metadata");
        let decoded: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded["freshness"], "current");
        assert_eq!(decoded["completeness"], "complete");
        assert_eq!(decoded["repository_knowledge"]["freshness"], "current");
        assert_eq!(decoded["repository_knowledge"]["completeness"], "complete");
    }
}
