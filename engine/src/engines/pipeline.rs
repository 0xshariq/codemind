use crate::{
    contract::{events::EngineEvent, operation::OperationState},
    engines::{
        chie::{AnalysisRun, ChangeSet, ChieEngine},
        coie::{CoieEngine, model::EntityKind},
        tie::{TieEngine, graph_builder as tie_graph_builder, mapping::target_mapping},
    },
    graph_storage::{GraphStorage, StorageError},
    lifecycle::Operation,
    source::SourceSnapshot,
};
use chrono::Utc;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PipelineRequest {
    pub snapshot: SourceSnapshot,
    pub selected_entity_id: String,
    pub request_id: String,
    pub trace_id: String,
    pub operation_id: Option<String>,
}

impl PipelineRequest {
    pub fn validate(&self) -> Result<(), PipelineError> {
        self.snapshot
            .validate()
            .map_err(|error| PipelineError::InvalidInput(error.to_string()))?;
        if self.selected_entity_id.trim().is_empty() {
            return Err(PipelineError::InvalidInput(
                "selected_entity_id is required".to_owned(),
            ));
        }
        if self.request_id.trim().is_empty() || self.trace_id.trim().is_empty() {
            return Err(PipelineError::InvalidInput(
                "request_id and trace_id are required".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub operation_id: String,
    pub graph: crate::engines::coie::model::CodeGraph,
    pub tests: Vec<crate::engines::tie::model::TestEntity>,
    pub test_relationships: Vec<crate::engines::tie::model::TestRelationship>,
    pub change_set: ChangeSet,
    pub analysis_run: AnalysisRun,
    pub analysis: crate::engines::chie::model::ChangeAnalysis,
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("invalid pipeline input: {0}")]
    InvalidInput(String),
    #[error("pipeline storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("pipeline engine error: {0}")]
    Engine(String),
}

/// The guaranteed hackathon investigation path.
///
/// The order is intentionally fixed:
///
/// `source snapshot -> CoIE -> Code Graph -> TIE -> Test Graph -> ChIE -> Change Graph`.
///
/// Each stage consumes the validated output of the previous stage. No stage
/// silently fabricates missing upstream facts.
pub struct InvestigationPipeline<S> {
    storage: Arc<S>,
    coie: CoieEngine,
    tie: TieEngine,
    chie: ChieEngine,
}

impl<S> InvestigationPipeline<S>
where
    S: GraphStorage + 'static,
{
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            coie: CoieEngine,
            tie: TieEngine,
            chie: ChieEngine::default(),
        }
    }

    pub async fn run(&self, request: PipelineRequest) -> Result<PipelineResult, PipelineError> {
        request.validate()?;
        let operation_id = request
            .operation_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("operation:{}", Uuid::new_v4()));
        let mut operation = Operation::new(
            operation_id.clone(),
            request.request_id.clone(),
            request.trace_id.clone(),
            request.snapshot.context.clone(),
        )
        .map_err(PipelineError::Engine)?;
        self.storage.persist_operation(&operation.record).await?;

        let result = self.run_stages(&request, &mut operation).await;
        match result {
            Ok(result) => {
                operation.record.result_reference =
                    Some(format!("change-analysis:{}", result.analysis_run.id));
                operation
                    .transition(OperationState::Completed)
                    .map_err(PipelineError::Engine)?;
                self.storage.persist_operation(&operation.record).await?;
                let event = operation.completed_event().map_err(PipelineError::Engine)?;
                self.storage.append_operation_event(&event).await?;
                Ok(result)
            }
            Err(error) => {
                operation.record.error_code = Some(error_code(&error).to_owned());
                if operation.record.state != OperationState::Failed {
                    let _ = operation.transition(OperationState::Failed);
                }
                let _ = self.storage.persist_operation(&operation.record).await;
                let failed = operation.event(
                    "pipeline".to_owned(),
                    crate::contract::operation::StageState::Failed,
                    error.to_string(),
                );
                if let Ok(event) = failed {
                    let _ = self.persist_operation_event(&event).await;
                }
                Err(error)
            }
        }
    }

    async fn run_stages(
        &self,
        request: &PipelineRequest,
        operation: &mut Operation,
    ) -> Result<PipelineResult, PipelineError> {
        self.stage_start(operation, "coie").await?;
        let graph = self
            .coie
            .extract(&request.snapshot)
            .map_err(|error| PipelineError::Engine(error.to_string()))?;
        crate::contract::intelligence::CoieResult {
            graph: graph.clone(),
        }
        .validate(&request.snapshot.context)
        .map_err(|error| PipelineError::Engine(error.to_string()))?;
        self.storage.write_code_graph(&graph).await?;
        self.stage_finish(operation, "coie").await?;

        self.stage_start(operation, "tie").await?;
        let (tests, relationships) = self
            .tie
            .discover(&request.snapshot)
            .map_err(|error| PipelineError::Engine(error.to_string()))?;
        let (tests, relationships) = target_mapping::resolve_graph(&graph, tests, relationships);
        tie_graph_builder::validate_against_code_graph(&tests, &relationships, &graph)
            .map_err(PipelineError::Engine)?;
        crate::contract::intelligence::TieResult {
            tests: tests.clone(),
            relationships: relationships.clone(),
        }
        .validate(&request.snapshot.context)
        .map_err(|error| PipelineError::Engine(error.to_string()))?;
        self.storage
            .write_test_graph(&request.snapshot.context, &tests, &relationships)
            .await?;
        self.stage_finish(operation, "tie").await?;

        self.stage_start(operation, "chie").await?;
        let selected = graph
            .entities
            .iter()
            .find(|entity| entity.id == request.selected_entity_id)
            .ok_or_else(|| {
                PipelineError::InvalidInput("selected entity is absent from CoIE".to_owned())
            })?;
        if !matches!(selected.kind, EntityKind::Symbol | EntityKind::File) {
            return Err(PipelineError::InvalidInput(
                "selected entity must be a symbol or file".to_owned(),
            ));
        }

        // ChIE consumes a bounded, storage-backed projection instead of the
        // in-memory CoIE/TIE outputs. This keeps FalkorDB load-bearing in the
        // real pipeline and exercises the same Type-B-style semantic reads
        // used by the transport boundary.
        let paths = self
            .storage
            .dependency_paths_for(
                &request.snapshot.context,
                &request.selected_entity_id,
                3,
                100,
            )
            .await?;
        let mut relationship_ids = std::collections::HashSet::new();
        let mut bounded_relationships = Vec::new();
        let mut entity_ids = std::collections::HashSet::from([request.selected_entity_id.clone()]);
        for path in paths {
            for relationship in path.relationships {
                if !matches!(
                    relationship.kind,
                    crate::engines::coie::model::RelationshipKind::Calls
                        | crate::engines::coie::model::RelationshipKind::Imports
                ) {
                    continue;
                }
                entity_ids.insert(relationship.from.clone());
                entity_ids.insert(relationship.to.clone());
                if relationship_ids.insert(relationship.id.clone()) {
                    bounded_relationships.push(relationship);
                }
            }
        }
        let mut bounded_entities = Vec::new();
        for entity_id in &entity_ids {
            let entity = self
                .storage
                .entity_for(&request.snapshot.context, entity_id)
                .await?
                .ok_or_else(|| {
                    PipelineError::Storage(StorageError::Query(format!(
                        "storage-backed ChIE entity missing: {entity_id}"
                    )))
                })?;
            bounded_entities.push(entity);
        }
        let bounded_graph = crate::engines::coie::model::CodeGraph {
            context: request.snapshot.context.clone(),
            entities: bounded_entities,
            relationships: bounded_relationships,
            diagnostics: Vec::new(),
        };
        let target_ids: Vec<String> = entity_ids.iter().cloned().collect();
        let stored_test_graph = self
            .storage
            .tests_for_context(&request.snapshot.context, &target_ids, 100)
            .await?;
        let analysis = self
            .chie
            .analyze(
                &bounded_graph,
                &stored_test_graph.tests,
                &stored_test_graph.relationships,
                &request.selected_entity_id,
            )
            .map_err(PipelineError::Engine)?;
        let mut change_set = ChangeSet::for_entity(
            format!("change-set:{}", operation.record.operation_id),
            request.snapshot.context.clone(),
            request.selected_entity_id.clone(),
            selected.kind.clone(),
            selected.evidence.clone(),
        )
        .map_err(PipelineError::Engine)?;
        let change_record = self
            .chie
            .change_record(&change_set.id, &graph, &request.selected_entity_id)
            .map_err(PipelineError::Engine)?;
        let started_at = Utc::now().to_rfc3339();
        let analysis_run = AnalysisRun {
            id: format!("analysis-run:{}", operation.record.operation_id),
            change_set_id: change_set.id.clone(),
            context: request.snapshot.context.clone(),
            status: "completed".to_owned(),
            started_at: started_at.clone(),
            completed_at: Some(Utc::now().to_rfc3339()),
        };
        analysis_run.validate().map_err(PipelineError::Engine)?;
        change_set.analysis_run_id = Some(analysis_run.id.clone());
        self.storage
            .persist_change_analysis(&change_set, &change_record, &analysis_run, &analysis)
            .await?;
        let graph_updated = operation.graph_updated().map_err(PipelineError::Engine)?;
        self.storage.append_operation_event(&graph_updated).await?;
        self.stage_finish(operation, "chie").await?;

        Ok(PipelineResult {
            operation_id: operation.record.operation_id.clone(),
            graph,
            tests: stored_test_graph.tests,
            test_relationships: stored_test_graph.relationships,
            change_set,
            analysis_run,
            analysis,
        })
    }

    async fn stage_start(
        &self,
        operation: &mut Operation,
        stage: &str,
    ) -> Result<(), PipelineError> {
        let event = operation
            .begin_stage(stage)
            .map_err(PipelineError::Engine)?;
        self.storage.persist_operation(&operation.record).await?;
        self.persist_operation_event(&event).await
    }

    async fn stage_finish(
        &self,
        operation: &mut Operation,
        stage: &str,
    ) -> Result<(), PipelineError> {
        let event = operation
            .finish_stage(stage)
            .map_err(PipelineError::Engine)?;
        self.storage.persist_operation(&operation.record).await?;
        self.persist_operation_event(&event).await
    }

    async fn persist_operation_event(
        &self,
        event: &crate::lifecycle::operation::OperationEvent,
    ) -> Result<(), PipelineError> {
        let engine_event = EngineEvent::Progress {
            metadata: event.metadata.clone(),
            engine: "codemind".to_owned(),
            stage: event.stage.clone().unwrap_or_else(|| "pipeline".to_owned()),
            message: event.message.clone().unwrap_or_default(),
        };
        self.storage.append_operation_event(&engine_event).await?;
        Ok(())
    }
}

fn error_code(error: &PipelineError) -> &'static str {
    match error {
        PipelineError::InvalidInput(_) => "INVALID_INPUT",
        PipelineError::Storage(_) => "STORAGE_ERROR",
        PipelineError::Engine(_) => "ENGINE_ERROR",
    }
}
