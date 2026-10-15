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
