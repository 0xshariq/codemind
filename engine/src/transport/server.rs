use crate::memory::{
    model::{EvidenceReference, HandoffRecord, MemoryRecord, ReviewContextRecord},
    scope::AccessContext,
    service,
};
use crate::{
    contract::generated::{
        self, ChangeAnalysisQuery, QueryResponse, SymbolRelationshipQuery,
        intelligence_service_server::IntelligenceService, query_response,
        repository_service_server::RepositoryService,
    },
    engines::{chie::ChieEngine, coie::model::CodeGraph},
    graph_storage::{GraphStorage, StorageError},
    source::RepositoryContext,
};
use futures::Stream;
use std::{pin::Pin, sync::Arc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

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
    provider: crate::source::SourceProvider,
}

impl<S> OperationServer<S> {
    pub fn new(storage: Arc<S>, provider: crate::source::SourceProvider) -> Self {
        Self { storage, provider }
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
        let snapshot = self
            .provider
            .resolve_with_repository_identity(
                &input.source_path,
                &context.workspace_id,
                (!context.repository_id.is_empty()).then_some(context.repository_id.as_str()),
            )
            .map_err(|error| Status::failed_precondition(error.to_string()))?;
        let graph = crate::engines::coie::CoieEngine
            .extract(&snapshot)
            .map_err(|error| Status::failed_precondition(error.to_string()))?;
        let selected = graph
            .entities
            .iter()
            .filter(|entity| {
                matches!(
                    entity.kind,
                    crate::engines::coie::model::EntityKind::Symbol
                        | crate::engines::coie::model::EntityKind::File
                ) && entity.name == input.symbol_name
                    && entity.context == snapshot.context
            })
            .collect::<Vec<_>>();
        let selected = match selected.as_slice() {
            [selected] => selected,
            [] => return Err(Status::not_found("selected symbol or file was not found")),
            _ => {
                return Err(Status::failed_precondition(
                    "selected symbol or file is ambiguous",
                ));
            }
        };

        let operation_id = format!("operation:{}", uuid::Uuid::new_v4());
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
        let initial_operation = crate::lifecycle::Operation::new(
            operation_id.clone(),
            request_id.clone(),
            trace_id.clone(),
            snapshot.context.clone(),
        )
        .map_err(Status::internal)?;
        self.storage
            .persist_operation(&initial_operation.record)
            .await
            .map_err(storage_status)?;

        let pipeline =
            crate::engines::pipeline::InvestigationPipeline::new(Arc::clone(&self.storage));
        let pipeline_request = crate::engines::pipeline::PipelineRequest {
            snapshot: snapshot.clone(),
            selected_entity_id: selected.id.clone(),
            request_id,
            trace_id,
            operation_id: Some(operation_id.clone()),
        };
        let pipeline_operation_id = operation_id.clone();
        tokio::spawn(async move {
            if let Err(error) = pipeline.run(pipeline_request).await {
                tracing::error!(
                    %pipeline_operation_id,
                    %error,
                    "CodeMind investigation pipeline failed"
                );
            }
        });

        Ok(Response::new(generated::StartInvestigationResponse {
            operation_id,
            context: Some(crate::transport::conversions::context_proto(
                &snapshot.context,
            )),
            selected_entity_id: selected.id.clone(),
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

#[derive(Default)]
pub struct RepositoryServer {
    provider: crate::source::SourceProvider,
}

#[tonic::async_trait]
impl RepositoryService for RepositoryServer {
    async fn resolve_repository(
        &self,
        request: Request<generated::ResolveRepositoryRequest>,
    ) -> Result<Response<generated::ResolveRepositoryResponse>, Status> {
        let input = request.into_inner();
        let context = input
            .context
            .ok_or_else(|| Status::invalid_argument("context is required"))?;
        let workspace_id = if context.workspace_id.trim().is_empty() {
            return Err(Status::invalid_argument("workspace_id is required"));
        } else {
            context.workspace_id
        };
        if input.source_path.trim().is_empty() {
            return Err(Status::invalid_argument("source_path is required"));
        }
        let repository_identity =
            (!context.repository_id.trim().is_empty()).then_some(context.repository_id.as_str());
        let snapshot = self
            .provider
            .resolve_with_repository_identity(&input.source_path, workspace_id, repository_identity)
            .map_err(|error| Status::failed_precondition(error.to_string()))?;
        let repository_name = snapshot
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("repository")
            .to_owned();
        let manifest_json = serde_json::to_string(&snapshot.manifest).map_err(|error| {
            Status::internal(format!("failed to serialize repository manifest: {error}"))
        })?;
        Ok(Response::new(generated::ResolveRepositoryResponse {
            result: Some(generated::resolve_repository_response::Result::Snapshot(
                generated::RepositorySnapshot {
                    context: Some(crate::transport::conversions::context_proto(
                        &snapshot.context,
                    )),
                    repository_name,
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
    Ok(RepositoryContext {
        repository_id: input.repository_id,
        workspace_id: input.workspace_id,
        source_snapshot_id: input.source_snapshot_id,
        graph_version: input.graph_version,
    })
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
    async fn query_symbol_relationships(
        &self,
        request: Request<SymbolRelationshipQuery>,
    ) -> Result<Response<QueryResponse>, Status> {
        let input = request.into_inner();
        let context = repository_context(
            input
                .context
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        let requested_limit = input.limit.clamp(1, 100) as usize;
        let mut relationships = self
            .storage
            .relationships_for(&context, &input.symbol_id, requested_limit + 1)
            .await
            .map_err(storage_status)?;
        let truncated = relationships.len() > requested_limit;
        relationships.truncate(requested_limit);

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
                freshness: generated::Freshness::Current as i32,
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
                limitations: if truncated {
                    vec!["relationship result was truncated at the requested bound".to_owned()]
                } else {
                    Vec::new()
                },
            })),
        }))
    }

    async fn analyze_change(
        &self,
        request: Request<ChangeAnalysisQuery>,
    ) -> Result<Response<QueryResponse>, Status> {
        let input = request.into_inner();
        let context = repository_context(
            input
                .context
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        if input.max_depth == 0 || input.max_depth > 8 {
            return Err(Status::invalid_argument(
                "max_depth must be between 1 and 8",
            ));
        }

        let selected = self
            .storage
            .entity_for(&context, &input.symbol_id)
            .await
            .map_err(storage_status)?
            .ok_or_else(|| Status::not_found("selected symbol or file was not found"))?;

        let dependency_paths = self
            .storage
            .dependency_paths_for(
                &context,
                &input.symbol_id,
                input.max_depth as usize,
                crate::contract::result::MAX_PATHS,
            )
            .await
            .map_err(storage_status)?;
        let paths = dependency_paths.paths;

        let mut entity_ids = vec![selected.id.clone()];
        let mut relationships = Vec::new();
        for path in &paths {
            if !entity_ids.iter().any(|id| id == &path.terminal_entity_id) {
                entity_ids.push(path.terminal_entity_id.clone());
            }
            relationships.extend(path.relationships.clone());
            for relationship in &path.relationships {
                if !entity_ids.iter().any(|id| id == &relationship.from) {
                    entity_ids.push(relationship.from.clone());
                }
                if !entity_ids.iter().any(|id| id == &relationship.to) {
                    entity_ids.push(relationship.to.clone());
                }
            }
        }

        let mut entities = Vec::new();
        for id in entity_ids {
            if let Some(entity) = self
                .storage
                .entity_for(&context, &id)
                .await
                .map_err(storage_status)?
            {
                entities.push(entity);
            }
        }

        let target_ids = paths
            .iter()
            .map(|path| path.terminal_entity_id.clone())
            .collect::<Vec<_>>();
        let stored_tests = self
            .storage
            .tests_for_context(
                &context,
                &target_ids,
                crate::contract::result::MAX_RELATIONSHIPS,
            )
            .await
            .map_err(storage_status)?;

        let graph = CodeGraph {
            context: context.clone(),
            entities,
            relationships,
            diagnostics: Vec::new(),
        };

        let mut analysis = ChieEngine {
            max_depth: input.max_depth as usize,
        }
        .analyze(
            &graph,
            &stored_tests.tests,
            &stored_tests.relationships,
            &selected.id,
        )
        .map_err(Status::failed_precondition)?;
        if dependency_paths.truncated {
            analysis.truncated = true;
            if let Some(impact) = analysis.impact.as_mut() {
                impact.truncated = true;
                impact.availability = crate::engines::chie::model::EvidenceAvailability::Partial;
            }
            analysis.limitations.push(
                "Storage dependency paths were truncated at the bounded read limit.".to_owned(),
            );
        }

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
                freshness: generated::Freshness::Current as i32,
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

fn storage_status(error: StorageError) -> Status {
    match error {
        StorageError::InvalidContext => Status::invalid_argument("invalid repository context"),
        StorageError::Unavailable(message) => Status::unavailable(message),
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
