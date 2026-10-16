use crate::memory::{
    model::{EvidenceReference, HandoffRecord, MemoryRecord, ReviewContextRecord},
    service,
};
use crate::{
    contract::generated::{
        self, ChangeAnalysisQuery, QueryResponse, SymbolRelationshipQuery,
        intelligence_service_server::IntelligenceService, query_response,
    },
    graph_storage::{GraphStorage, StorageError},
    source::RepositoryContext,
};
use std::sync::Arc;
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

#[tonic::async_trait]
impl<S> generated::memory_service_server::MemoryService for MemoryServer<S>
where
    S: crate::memory::repository::MemoryRepository + 'static,
{
    async fn persist_memory(
        &self,
        request: Request<generated::PersistMemoryRequest>,
    ) -> Result<Response<generated::MemoryResponse>, Status> {
        let input = request
            .into_inner()
            .memory
            .ok_or_else(|| Status::invalid_argument("memory is required"))?;
        let record = memory_record(input)?;
        let saved = service::persist_memory(self.storage.as_ref(), record)
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
        let input = request.into_inner();
        let context = repository_context(
            input
                .context
                .ok_or_else(|| Status::invalid_argument("context is required"))?,
        )?;
        let records = service::retrieve_memory(
            self.storage.as_ref(),
            &context,
            &input.scope,
            &input.kind,
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
            ))),
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
            ))),
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
            ))),
        }))
    }

    async fn persist_review_context(
        &self,
        request: Request<generated::PersistReviewContextRequest>,
    ) -> Result<Response<generated::MemoryResponse>, Status> {
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
        let record =
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
        let saved = service::persist_review_context(self.storage.as_ref(), record)
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
        .map(|mut record: HandoffRecord| {
            record.handoff_id = input.handoff_id;
            record.status = input.status;
            record.context = context;
            record
        })
}

fn memory_proto(record: MemoryRecord) -> generated::MemoryRecord {
    generated::MemoryRecord {
        memory_id: record.memory_id,
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

fn handoff_proto(record: HandoffRecord) -> generated::Handoff {
    let payload_json = serde_json::to_string(&record).unwrap_or_default();
    generated::Handoff {
        handoff_id: record.handoff_id,
        status: record.status,
        payload_json,
        context: Some(generated::RepositoryContext {
            repository_id: record.context.repository_id,
            workspace_id: record.context.workspace_id,
            source_snapshot_id: record.context.source_snapshot_id,
            graph_version: record.context.graph_version,
        }),
    }
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
        let context = input
            .context
            .ok_or_else(|| Status::invalid_argument("context is required"))?;
        let context = RepositoryContext {
            repository_id: context.repository_id.clone(),
            workspace_id: context.workspace_id.clone(),
            source_snapshot_id: context.source_snapshot_id.clone(),
            graph_version: context.graph_version.clone(),
        };
        let relationships = self
            .storage
            .relationships_for(
                &context,
                &input.symbol_id,
                input.limit.clamp(1, 100) as usize,
            )
            .await
            .map_err(storage_status)?;
        let data = serde_json::to_string(&relationships)
            .map_err(|error| Status::internal(error.to_string()))?;
        Ok(Response::new(QueryResponse {
            result: Some(query_response::Result::Data(generated::CanonicalResult {
                outcome: if relationships.is_empty() {
                    generated::Outcome::Empty as i32
                } else {
                    generated::Outcome::Success as i32
                },
                completeness: generated::Completeness::Complete as i32,
                freshness: generated::Freshness::Current as i32,
                context: Some(generated::RepositoryContext {
                    repository_id: context.repository_id,
                    workspace_id: context.workspace_id,
                    source_snapshot_id: context.source_snapshot_id,
                    graph_version: context.graph_version,
                }),
                correlation: None,
                data_json: data,
                evidence: Vec::new(),
                diagnostics: Vec::new(),
                limitations: vec!["Day 1 query returns one-hop outgoing relationships".to_owned()],
            })),
        }))
    }

    async fn analyze_change(
        &self,
        request: Request<ChangeAnalysisQuery>,
    ) -> Result<Response<QueryResponse>, Status> {
        let input = request.into_inner();
        let context = input
            .context
            .ok_or_else(|| Status::invalid_argument("context is required"))?;
        let context = RepositoryContext {
            repository_id: context.repository_id,
            workspace_id: context.workspace_id,
            source_snapshot_id: context.source_snapshot_id,
            graph_version: context.graph_version,
        };
        let analysis = self
            .storage
            .analyze_change(&context, &input.symbol_id, input.max_depth as usize)
            .await
            .map_err(storage_status)?;
        let data_json = serde_json::to_string(&analysis)
            .map_err(|error| Status::internal(error.to_string()))?;
        Ok(Response::new(QueryResponse {
            result: Some(query_response::Result::Data(generated::CanonicalResult {
                outcome: if analysis.truncated {
                    generated::Outcome::Partial as i32
                } else {
                    generated::Outcome::Success as i32
                },
                completeness: if analysis.truncated {
                    generated::Completeness::Truncated as i32
                } else {
                    generated::Completeness::Complete as i32
                },
                freshness: generated::Freshness::Current as i32,
                context: Some(generated::RepositoryContext {
                    repository_id: analysis.context.repository_id,
                    workspace_id: analysis.context.workspace_id,
                    source_snapshot_id: analysis.context.source_snapshot_id,
                    graph_version: analysis.context.graph_version,
                }),
                correlation: None,
                data_json,
                evidence: Vec::new(),
                diagnostics: Vec::new(),
                limitations: analysis.limitations,
            })),
        }))
    }
}

fn storage_status(error: StorageError) -> Status {
    match error {
        StorageError::InvalidContext => Status::invalid_argument("invalid repository context"),
        StorageError::Unavailable(message) => Status::unavailable(message),
        StorageError::Query(message) => Status::internal(message),
    }
}
