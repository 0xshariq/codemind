use crate::{
    contract::{
        events::EngineEvent,
        intelligence::{ChangeAnalysisRequest, CoieQueryKind, CoieQueryRequest},
        operation::OperationState,
    },
    engines::{
        chie::{AnalysisRun, ChangeSet, ChieEngine},
        coie::{
            CoieEngine,
            model::{CodeGraph, EntityKind, GraphEntity, GraphRelationship},
        },
        tie::{TieEngine, graph_builder as tie_graph_builder, mapping::target_mapping},
    },
    graph_storage::{
        ChangeGraphVerificationExpectation, DerivedRelationship, GraphStorage, StorageError,
    },
    lifecycle::Operation,
    repository::KnowledgeCompleteness,
    source::{
        RepositoryContext, SourceProvider, SourceSnapshot,
        identity::{RandomIdGenerator, derive_sha256_id},
    },
};
use chrono::Utc;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Request envelope for the compatibility snapshot-based investigation path.
#[derive(Debug, Clone)]
pub struct PipelineRequest {
    pub snapshot: SourceSnapshot,
    pub selected_entity_id: String,
    pub request_id: String,
    pub trace_id: String,
    pub operation_id: Option<String>,
    /// CoIE graph and selected ID produced by the same extraction pass.
    ///
    /// Required for the snapshot-based `run` entry point so selection cannot
    /// refer to IDs from a different extraction of the same snapshot.
    pub prepared_selection: Option<PreparedSelection>,
}

impl PipelineRequest {
    /// Validate the snapshot, selected entity and prepared CoIE extraction contract.
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
        let selection = self.prepared_selection.as_ref().ok_or_else(|| {
            PipelineError::InvalidInput(
                "snapshot-based run requires a prepared CoIE selection; call prepare_selection first"
                    .to_owned(),
            )
        })?;
        if selection.context != self.snapshot.context
            || selection.graph.context != self.snapshot.context
            || selection.selected_entity_id != self.selected_entity_id
        {
            return Err(PipelineError::InvalidInput(
                "prepared selection does not match the pipeline snapshot and selected entity"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

fn error_code(error: &PipelineError) -> &'static str {
    match error {
        PipelineError::InvalidInput(_) => "INVALID_INPUT",
        PipelineError::SelectedEntityNotFound => "SELECTED_ENTITY_NOT_FOUND",
        PipelineError::SelectedEntityAmbiguous => "SELECTED_ENTITY_AMBIGUOUS",
        PipelineError::Storage(_) => "STORAGE_ERROR",
        PipelineError::Engine(_) => "ENGINE_ERROR",
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedPipelineRequest {
    pub context: crate::source::RepositoryContext,
    pub selected_entity_id: String,
    pub request_id: String,
    pub trace_id: String,
    pub operation_id: String,
    pub knowledge_completeness: KnowledgeCompleteness,
    pub freshness: crate::contract::result::Freshness,
    pub omitted_file_count: u64,
    pub history_complete: bool,
    pub history_omitted_commits_at_least: u64,
    pub history_omitted_path_changes_at_least: u64,
}

impl VerifiedPipelineRequest {
    /// Validate the pinned repository context and the completeness/freshness claims.
    pub fn validate(&self) -> Result<(), PipelineError> {
        self.context
            .validate()
            .map_err(|error| PipelineError::InvalidInput(error.to_string()))?;
        if self.selected_entity_id.trim().is_empty() {
            return Err(PipelineError::InvalidInput(
                "selected_entity_id is required".to_owned(),
            ));
        }
        if self.request_id.trim().is_empty()
            || self.trace_id.trim().is_empty()
            || self.operation_id.trim().is_empty()
        {
            return Err(PipelineError::InvalidInput(
                "request_id, trace_id and operation_id are required".to_owned(),
            ));
        }
        if self.knowledge_completeness == KnowledgeCompleteness::Unknown {
            return Err(PipelineError::InvalidInput(
                "verified investigation requires known repository completeness".to_owned(),
            ));
        }
        if self.history_complete
            && (self.history_omitted_commits_at_least != 0
                || self.history_omitted_path_changes_at_least != 0)
        {
            return Err(PipelineError::InvalidInput(
                "complete Git history cannot report omitted commits or path changes".to_owned(),
            ));
        }
        if self.knowledge_completeness == KnowledgeCompleteness::Complete
            && !repository_completeness_claim_is_consistent(
                self.omitted_file_count,
                self.history_complete,
                self.history_omitted_commits_at_least,
                self.history_omitted_path_changes_at_least,
            )
        {
            return Err(PipelineError::InvalidInput(
                "complete repository knowledge cannot report source or Git-history omissions"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub operation_id: String,
    pub knowledge_completeness: KnowledgeCompleteness,
    pub freshness: crate::contract::result::Freshness,
    pub omitted_file_count: u64,
    pub history_complete: bool,
    pub history_omitted_commits_at_least: u64,
    pub history_omitted_path_changes_at_least: u64,
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
    #[error("selected symbol or file was not found")]
    SelectedEntityNotFound,
    #[error("selected symbol or file is ambiguous")]
    SelectedEntityAmbiguous,
    #[error("pipeline storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("pipeline engine error: {0}")]
    Engine(String),
}

/// A selected entity and the exact CoIE graph from which its ID was obtained.
#[derive(Debug, Clone)]
pub struct PreparedSelection {
    context: RepositoryContext,
    graph: CodeGraph,
    selected_entity_id: String,
}

impl PreparedSelection {
    /// Return the entity ID selected from this prepared CoIE graph.
    pub fn selected_entity_id(&self) -> &str {
        &self.selected_entity_id
    }
}

#[derive(Debug, Clone)]
pub struct CodeSearchResult {
    pub matches: Vec<GraphEntity>,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct RelationshipQueryResult {
    pub relationships: Vec<GraphRelationship>,
    pub truncated: bool,
}

/// Question-specific investigation engine.
///
/// The snapshot-based `run` path remains for compatibility and performs the
/// full CoIE/TIE/ChIE sequence. Production repository ingestion is owned by
/// `RepositoryIndexPipeline`; after its graph verification, `run_verified`
/// performs only the storage-backed, question-specific ChIE stage.
///
/// Each path consumes validated upstream facts. No stage silently fabricates
/// missing repository knowledge.
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
    /// Construct an investigation pipeline over the shared graph storage adapter.
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            coie: CoieEngine,
            tie: TieEngine,
            chie: ChieEngine::default(),
        }
    }

    /// Compatibility entry point for local snapshot-based callers. Capture
    /// the source through the provider, extract CoIE once, and resolve the
    /// selected symbol/file from that same graph. The graph is then passed to
    /// `run_with_prepared_selection` so it is not extracted a second time.
    pub fn prepare_source_selection(
        &self,
        provider: &SourceProvider,
        source_path: &str,
        workspace_id: &str,
        repository_identity: Option<&str>,
        selected_name: &str,
    ) -> Result<(SourceSnapshot, PreparedSelection), PipelineError> {
        let snapshot = provider
            .resolve_with_repository_identity(source_path, workspace_id, repository_identity)
            .map_err(|error| PipelineError::Engine(error.to_string()))?;
        let selection = self.prepare_selection(&snapshot, selected_name)?;
        Ok((snapshot, selection))
    }

    /// Extract CoIE once and resolve a symbol or file against that exact graph.
    pub fn prepare_selection(
        &self,
        snapshot: &SourceSnapshot,
        selected_name: &str,
    ) -> Result<PreparedSelection, PipelineError> {
        snapshot
            .validate()
            .map_err(|error| PipelineError::InvalidInput(error.to_string()))?;
        let graph = self
            .coie
            .extract(snapshot)
            .map_err(|error| PipelineError::Engine(error.to_string()))?;
        crate::contract::intelligence::CoieResult {
            graph: graph.clone(),
        }
        .validate(&snapshot.context)
        .map_err(|error| PipelineError::Engine(error.to_string()))?;
        let selected_entity_id = select_entity_id(
            graph.entities.iter().map(|entity| {
                (
                    entity.id.as_str(),
                    entity.name.as_str(),
                    matches!(entity.kind, EntityKind::Symbol | EntityKind::File),
                )
            }),
            selected_name,
        )?
        .to_owned();

        Ok(PreparedSelection {
            context: snapshot.context.clone(),
            graph,
            selected_entity_id,
        })
    }

    /// Run the legacy snapshot-based pipeline using a graph already extracted
    /// by `prepare_selection`. This keeps entity selection and graph indexing
    /// on one CoIE result rather than parsing the same snapshot twice.
    pub async fn run_with_prepared_selection(
        &self,
        request: PipelineRequest,
        selection: PreparedSelection,
    ) -> Result<PipelineResult, PipelineError> {
        let mut request = request;
        request.prepared_selection = Some(selection);
        self.run(request).await
    }

    /// Execute bounded read-only intelligence queries behind the engine
    /// boundary. Transport adapters only validate/map protobuf envelopes and
    /// serialize the returned domain values.
    pub async fn search_code(
        &self,
        request: CoieQueryRequest,
    ) -> Result<CodeSearchResult, PipelineError> {
        request
            .validate()
            .map_err(|error| PipelineError::InvalidInput(error.to_string()))?;
        if request.kind != CoieQueryKind::SearchSymbols {
            return Err(PipelineError::InvalidInput(
                "code search requires a SearchSymbols CoIE query".to_owned(),
            ));
        }
        validate_repository_context(&request.context)?;
        let query = validate_search_query(&request.query)?;
        let limit = bounded_query_limit(request.limit);
        let mut matches = self
            .storage
            .search_code_entities(&request.context, query, limit + 1)
            .await?;
        let truncated = matches.len() > limit;
        matches.truncate(limit);
        Ok(CodeSearchResult { matches, truncated })
    }

    /// Execute a bounded, context-scoped CoIE relationship query.
    pub async fn query_symbol_relationships(
        &self,
        request: CoieQueryRequest,
    ) -> Result<RelationshipQueryResult, PipelineError> {
        request
            .validate()
            .map_err(|error| PipelineError::InvalidInput(error.to_string()))?;
        if request.kind != CoieQueryKind::QueryRelationships {
            return Err(PipelineError::InvalidInput(
                "relationship query requires a QueryRelationships CoIE query".to_owned(),
            ));
        }
        validate_repository_context(&request.context)?;
        validate_symbol_id(&request.query)?;
        let limit = bounded_query_limit(request.limit);
        let mut relationships = self
            .storage
            .relationships_for(&request.context, &request.query, limit + 1)
            .await?;
        let truncated = relationships.len() > limit;
        relationships.truncate(limit);
        Ok(RelationshipQueryResult {
            relationships,
            truncated,
        })
    }

    /// Analyze a selected entity using bounded, storage-backed graph evidence.
    pub async fn analyze_change_query(
        &self,
        request: ChangeAnalysisRequest,
    ) -> Result<crate::engines::chie::model::ChangeAnalysis, PipelineError> {
        request
            .validate()
            .map_err(|error| PipelineError::InvalidInput(error.to_string()))?;
        validate_repository_context(&request.context)?;
        validate_symbol_id(&request.target_entity_id)?;
        validate_change_depth(request.max_depth)?;

        let selected = self
            .storage
            .entity_for(&request.context, &request.target_entity_id)
            .await?
            .ok_or(PipelineError::SelectedEntityNotFound)?;
        let dependency_paths = self
            .storage
            .dependency_paths_for(
                &request.context,
                &request.target_entity_id,
                request.max_depth,
                crate::contract::result::MAX_PATHS,
            )
            .await?;
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
            if let Some(entity) = self.storage.entity_for(&request.context, &id).await? {
                entities.push(entity);
            }
        }

        let target_ids = paths
            .iter()
            .map(|path| path.terminal_entity_id.clone())
            .collect::<Vec<_>>();
        let stored_tests = self
            .storage
            .tests_for_context(&request.context, &target_ids, request.max_results)
            .await?;
        let graph = CodeGraph {
            context: request.context.clone(),
            entities,
            relationships,
            diagnostics: Vec::new(),
        };

        let mut analysis = ChieEngine {
            max_depth: request.max_depth,
        }
        .analyze(
            &graph,
            &stored_tests.tests,
            &stored_tests.relationships,
            &selected.id,
        )
        .map_err(PipelineError::Engine)?;
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
        Ok(analysis)
    }

    /// Run the snapshot-based pipeline using the exact CoIE extraction that
    /// produced the selected entity ID. Call `prepare_selection` first and
    /// attach its result to `PipelineRequest::prepared_selection`.
    pub async fn run(&self, request: PipelineRequest) -> Result<PipelineResult, PipelineError> {
        request.validate()?;
        let graph = request
            .prepared_selection
            .as_ref()
            .expect("request validation requires a prepared selection")
            .graph
            .clone();
        self.run_internal(request, Some(graph)).await
    }

    async fn run_internal(
        &self,
        request: PipelineRequest,
        prepared_graph: Option<CodeGraph>,
    ) -> Result<PipelineResult, PipelineError> {
        request.validate()?;
        let operation_id = request
            .operation_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        tracing::info!(
            operation_id = %operation_id,
            repository_id = %request.snapshot.context.repository_id,
            selected_entity_id = %request.selected_entity_id,
            "investigation pipeline accepted"
        );
        let mut operation = Operation::new(
            operation_id.clone(),
            request.request_id.clone(),
            request.trace_id.clone(),
            request.snapshot.context.clone(),
        )
        .map_err(PipelineError::Engine)?;
        self.storage.persist_operation(&operation.record).await?;

        let result = self
            .run_stages(&request, &mut operation, prepared_graph)
            .await;
        match result {
            Ok(result) => {
                operation.record.result_reference =
                    Some(format!("change-analysis:{}", result.analysis_run.id));
                operation
                    .transition(OperationState::Completed)
                    .map_err(PipelineError::Engine)?;
                let event = operation.completed_event().map_err(PipelineError::Engine)?;
                self.storage.persist_operation(&operation.record).await?;
                self.storage.append_operation_event(&event).await?;
                tracing::info!(
                    operation_id = %operation_id,
                    impacted_entities = result.analysis.impact_paths.len(),
                    test_candidates = result.analysis.test_candidates.len(),
                    truncated = result.analysis.truncated,
                    "investigation pipeline completed"
                );
                Ok(result)
            }
            Err(error) => {
                operation.record.error_code = Some(error_code(&error).to_owned());
                if operation.record.state != OperationState::Failed {
                    let _ = operation.transition(OperationState::Failed);
                }
                let failed = operation.event(
                    "pipeline".to_owned(),
                    crate::contract::operation::StageState::Failed,
                    error.to_string(),
                );
                if let Ok(event) = failed {
                    let _ = self.storage.persist_operation(&operation.record).await;
                    let _ = self.persist_operation_event(&event).await;
                } else {
                    let _ = self.storage.persist_operation(&operation.record).await;
                }
                tracing::error!(operation_id = %operation_id, error = %error, "investigation pipeline failed");
                Err(error)
            }
        }
    }

    /// Run only the question-specific ChIE path against a repository graph that
    /// has already been indexed and verified. No source snapshot or CoIE/TIE
    /// extraction is required here, so an owned source clone may already have
    /// been deleted.
    pub async fn run_verified(
        &self,
        request: VerifiedPipelineRequest,
    ) -> Result<PipelineResult, PipelineError> {
        request.validate()?;
        let record = self
            .storage
            .retrieve_operation(&request.context, &request.operation_id)
            .await?;
        let mut operation = Operation { record };
        let chie_stage = investigation_stage_order()[0];
        if let Err(error) = self.stage_start(&mut operation, chie_stage).await {
            operation.record.error_code = Some(error_code(&error).to_owned());
            if operation.record.state != OperationState::Failed {
                let _ = operation.transition(OperationState::Failed);
            }
            if let Err(persist_error) = self.storage.persist_operation(&operation.record).await {
                tracing::error!(
                    operation_id = %operation.record.operation_id,
                    error = %persist_error,
                    "could not persist operation after investigation stage-start failure"
                );
            }
            return Err(error);
        }
        let mut id_generator = RandomIdGenerator::default();
        let result = async {
            let selected = self
                .storage
                .entity_for(&request.context, &request.selected_entity_id)
                .await?
                .ok_or_else(|| {
                    PipelineError::InvalidInput(
                        "selected entity is absent from verified graph".to_owned(),
                    )
                })?;
            if !matches!(selected.kind, EntityKind::Symbol | EntityKind::File) {
                return Err(PipelineError::InvalidInput(
                    "selected entity must be a symbol or file".to_owned(),
                ));
            }
            let dependency_paths = self
                .storage
                .dependency_paths_for(&request.context, &request.selected_entity_id, 3, 100)
                .await?;
            let mut relationship_ids = std::collections::HashSet::new();
            let mut bounded_relationships = Vec::new();
            let mut entity_ids =
                std::collections::HashSet::from([request.selected_entity_id.clone()]);
            for path in dependency_paths.paths.iter() {
                for relationship in &path.relationships {
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
                        bounded_relationships.push(relationship.clone());
                    }
                }
            }
            let mut bounded_entities = Vec::new();
            for entity_id in &entity_ids {
                let entity = self
                    .storage
                    .entity_for(&request.context, entity_id)
                    .await?
                    .ok_or_else(|| {
                        PipelineError::Storage(StorageError::Query(format!(
                            "storage-backed ChIE entity missing: {entity_id}"
                        )))
                    })?;
                bounded_entities.push(entity);
            }
            let bounded_graph = crate::engines::coie::model::CodeGraph {
                context: request.context.clone(),
                entities: bounded_entities,
                relationships: bounded_relationships,
                diagnostics: Vec::new(),
            };
            let target_ids: Vec<String> = entity_ids.iter().cloned().collect();
            let stored_test_graph = self
                .storage
                .tests_for_context(&request.context, &target_ids, 100)
                .await?;
            let mut analysis = self
                .chie
                .analyze(
                    &bounded_graph,
                    &stored_test_graph.tests,
                    &stored_test_graph.relationships,
                    &request.selected_entity_id,
                )
                .map_err(PipelineError::Engine)?;
            if stored_test_graph.truncated {
                mark_analysis_partial(&mut analysis);
                analysis.limitations.push(
                    "Stored Test Graph results were truncated at the bounded storage query limit."
                        .to_owned(),
                );
            }
            if repository_knowledge_requires_partial(
                request.knowledge_completeness == KnowledgeCompleteness::Complete,
                request.omitted_file_count,
                request.history_complete,
                request.history_omitted_commits_at_least,
                request.history_omitted_path_changes_at_least,
            ) {
                mark_analysis_partial(&mut analysis);
            }
            if request.knowledge_completeness == KnowledgeCompleteness::Partial {
                analysis.limitations.push(
                    "Repository knowledge is partial because one or more source or Git-history inputs were omitted or truncated.".to_owned(),
                );
            }
            if request.omitted_file_count > 0 {
                analysis.limitations.push(format!(
                    "Source capture omitted at least {} file(s); conclusions may not cover those inputs.",
                    request.omitted_file_count
                ));
            }
            if !request.history_complete {
                analysis.limitations.push(format!(
                    "Git history is incomplete (at least {} commit(s) and {} path change(s) omitted).",
                    request.history_omitted_commits_at_least,
                    request.history_omitted_path_changes_at_least
                ));
            }
            match request.freshness {
                crate::contract::result::Freshness::Current => {}
                crate::contract::result::Freshness::Stale => {
                    analysis.limitations.push(
                        "The repository source was newer than the verified graph at the final freshness check; this analysis uses the indexed commit and should not be treated as current repository state.".to_owned(),
                    );
                }
                crate::contract::result::Freshness::Unknown => {
                    analysis.limitations.push(
                        "Current repository freshness could not be proven; this analysis is based on the last verified snapshot.".to_owned(),
                    );
                }
            }
            if dependency_paths.truncated {
                analysis.truncated = true;
                if let Some(impact) = analysis.impact.as_mut() {
                    impact.truncated = true;
                    impact.availability =
                        crate::engines::chie::model::EvidenceAvailability::Partial;
                }
                analysis.limitations.push(
                    "Storage dependency paths were truncated at the bounded read limit.".to_owned(),
                );
            }
            let change_set = ChangeSet::for_entity(
                id_generator.next("change-set", "operation"),
                request.context.clone(),
                request.selected_entity_id.clone(),
                selected.kind.clone(),
                selected.evidence.clone(),
            )
            .map_err(PipelineError::Engine)?;
            let change_record = self
                .chie
                .change_record(&change_set.id, &bounded_graph, &request.selected_entity_id)
                .map_err(PipelineError::Engine)?;
            let analysis_run = AnalysisRun {
                id: id_generator.next("analysis-run", "operation"),
                change_set_id: change_set.id.clone(),
                context: request.context.clone(),
                status: "completed".to_owned(),
                started_at: Utc::now().to_rfc3339(),
                completed_at: Some(Utc::now().to_rfc3339()),
            };
            analysis_run.validate().map_err(PipelineError::Engine)?;
            self.storage
                .persist_change_analysis(&change_set, &change_record, &analysis_run, &analysis)
                .await?;
            self.verify_change_publication(&change_set, &change_record, &analysis_run, &analysis)
                .await?;
            Ok::<PipelineResult, PipelineError>(PipelineResult {
                operation_id: operation.record.operation_id.clone(),
                knowledge_completeness: request.knowledge_completeness,
                freshness: request.freshness,
                omitted_file_count: request.omitted_file_count,
                history_complete: request.history_complete,
                history_omitted_commits_at_least: request.history_omitted_commits_at_least,
                history_omitted_path_changes_at_least: request.history_omitted_path_changes_at_least,
                graph: bounded_graph,
                tests: stored_test_graph.tests,
                test_relationships: stored_test_graph.relationships,
                change_set,
                analysis_run,
                analysis,
            })
        }
        .await;
        match result {
            Ok(result) => {
                let graph_updated = operation.graph_updated().map_err(PipelineError::Engine)?;
                self.storage.append_operation_event(&graph_updated).await?;
                self.stage_finish(&mut operation, chie_stage).await?;
                operation.record.result_reference =
                    Some(format!("change-analysis:{}", result.analysis_run.id));
                operation
                    .transition(OperationState::Completed)
                    .map_err(PipelineError::Engine)?;
                let completed = operation.completed_event().map_err(PipelineError::Engine)?;
                self.storage.persist_operation(&operation.record).await?;
                self.storage.append_operation_event(&completed).await?;
                Ok(result)
            }
            Err(error) => {
                operation.record.error_code = Some(error_code(&error).to_owned());
                if operation.record.state != OperationState::Failed {
                    let _ = operation.transition(OperationState::Failed);
                }
                let failed = operation.event(
                    chie_stage.to_owned(),
                    crate::contract::operation::StageState::Failed,
                    error.to_string(),
                );
                if let Err(persist_error) = self.storage.persist_operation(&operation.record).await
                {
                    tracing::error!(
                        operation_id = %operation.record.operation_id,
                        error = %persist_error,
                        "could not persist failed investigation operation"
                    );
                }
                if let Ok(event) = failed
                    && let Err(event_error) = self.persist_operation_event(&event).await
                {
                    tracing::error!(
                        operation_id = %operation.record.operation_id,
                        error = %event_error,
                        "could not persist failed investigation operation event"
                    );
                }
                Err(error)
            }
        }
    }
    async fn run_stages(
        &self,
        request: &PipelineRequest,
        operation: &mut Operation,
        prepared_graph: Option<CodeGraph>,
    ) -> Result<PipelineResult, PipelineError> {
        let mut id_generator = RandomIdGenerator::default();
        if prepared_graph.is_some() {
            self.stage_start_with_message(
                operation,
                "coie",
                "CoIE extraction and symbol selection were prepared; persisting the graph",
            )
            .await?;
        } else {
            self.stage_start(operation, "coie").await?;
        }
        let graph = match prepared_graph {
            Some(graph) => graph,
            None => self
                .coie
                .extract(&request.snapshot)
                .map_err(|error| PipelineError::Engine(error.to_string()))?,
        };
        crate::contract::intelligence::CoieResult {
            graph: graph.clone(),
        }
        .validate(&request.snapshot.context)
        .map_err(|error| PipelineError::Engine(error.to_string()))?;
        let selected_before_persistence = graph
            .entities
            .iter()
            .find(|entity| entity.id == request.selected_entity_id)
            .cloned()
            .ok_or_else(|| {
                PipelineError::InvalidInput("selected entity is absent from prepared CoIE".to_owned())
            })?;
        let graph = self.storage.write_code_graph(&graph).await?;
        let selected = graph
            .entities
            .iter()
            .find(|entity| {
                entity.kind == selected_before_persistence.kind
                    && entity.path == selected_before_persistence.path
                    && entity.name == selected_before_persistence.name
                    && entity.native_kind == selected_before_persistence.native_kind
                    && entity.language == selected_before_persistence.language
                    && entity.range == selected_before_persistence.range
                    && entity.line == selected_before_persistence.line
                    && entity.evidence == selected_before_persistence.evidence
            })
            .ok_or_else(|| {
                PipelineError::InvalidInput(
                    "selected entity could not be resolved in the canonical persisted CoIE graph"
                        .to_owned(),
                )
            })?;
        let selected_entity_id = selected.id.clone();
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
        if !matches!(selected.kind, EntityKind::Symbol | EntityKind::File) {
            return Err(PipelineError::InvalidInput(
                "selected entity must be a symbol or file".to_owned(),
            ));
        }

        // ChIE consumes a bounded, storage-backed projection instead of the
        // in-memory CoIE/TIE outputs. This keeps FalkorDB load-bearing in the
        // real pipeline and exercises the same Type-B-style semantic reads
        // used by the transport boundary.
        let dependency_paths = self
            .storage
            .dependency_paths_for(
                &request.snapshot.context,
                &selected_entity_id,
                3,
                100,
            )
            .await?;
        let paths = dependency_paths.paths;
        let mut relationship_ids = std::collections::HashSet::new();
        let mut bounded_relationships = Vec::new();
        let mut entity_ids = std::collections::HashSet::from([selected_entity_id.clone()]);
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
        let mut analysis = self
            .chie
            .analyze(
                &bounded_graph,
                &stored_test_graph.tests,
                &stored_test_graph.relationships,
                &selected_entity_id,
            )
            .map_err(PipelineError::Engine)?;
        if stored_test_graph.truncated {
            mark_analysis_partial(&mut analysis);
            analysis.limitations.push(
                "Stored Test Graph results were truncated at the bounded storage query limit."
                    .to_owned(),
            );
        }
        let omitted_file_count = request.snapshot.manifest.omitted_files.len() as u64;
        mark_analysis_partial(&mut analysis);
        analysis.limitations.push(
            "The compatibility pipeline did not prove repository graph usability or live source freshness; treat this analysis as snapshot-scoped and not current.".to_owned(),
        );
        if omitted_file_count > 0 {
            analysis.limitations.push(format!(
                "Source capture omitted at least {} file(s); conclusions may not cover those inputs.",
                omitted_file_count
            ));
        }
        analysis.limitations.push(
            "Git history was not captured by the compatibility pipeline; history completeness is unknown.".to_owned(),
        );
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
        let mut change_set = ChangeSet::for_entity(
            id_generator.next("change-set", "operation"),
            request.snapshot.context.clone(),
            selected_entity_id.clone(),
            selected.kind.clone(),
            selected.evidence.clone(),
        )
        .map_err(PipelineError::Engine)?;
        let change_record = self
            .chie
            .change_record(&change_set.id, &graph, &selected_entity_id)
            .map_err(PipelineError::Engine)?;
        let started_at = Utc::now().to_rfc3339();
        let analysis_run = AnalysisRun {
            id: id_generator.next("analysis-run", "operation"),
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
        self.verify_change_publication(&change_set, &change_record, &analysis_run, &analysis)
            .await?;
        let graph_updated = operation.graph_updated().map_err(PipelineError::Engine)?;
        self.storage.append_operation_event(&graph_updated).await?;
        self.stage_finish(operation, "chie").await?;

        Ok(PipelineResult {
            operation_id: operation.record.operation_id.clone(),
            knowledge_completeness: KnowledgeCompleteness::Unknown,
            freshness: crate::contract::result::Freshness::Unknown,
            omitted_file_count,
            history_complete: false,
            history_omitted_commits_at_least: 0,
            history_omitted_path_changes_at_least: 0,
            graph,
            tests: stored_test_graph.tests,
            test_relationships: stored_test_graph.relationships,
            change_set,
            analysis_run,
            analysis,
        })
    }

    async fn verify_change_publication(
        &self,
        change_set: &ChangeSet,
        change_record: &crate::engines::chie::ChangeRecord,
        analysis_run: &AnalysisRun,
        analysis: &crate::engines::chie::model::ChangeAnalysis,
    ) -> Result<(), PipelineError> {
        let impact_id = derive_sha256_id("chie-impact", "analysis-run", &analysis_run.id);
        let risk_id = derive_sha256_id("chie-risk", "analysis-run", &analysis_run.id);
        let mut entity_ids = vec![
            change_set.id.clone(),
            derive_sha256_id(
                "chie-change-record",
                "change-set",
                &change_record.change_set_id,
            ),
            analysis_run.id.clone(),
            impact_id.clone(),
        ];
        if analysis.risk_assessment.is_some() {
            entity_ids.push(risk_id.clone());
        }
        entity_ids.extend(analysis.evidence_records.iter().map(|e| e.id.clone()));
        entity_ids.sort_unstable();
        entity_ids.dedup();

        let mut relationships = vec![
            DerivedRelationship {
                kind: "ANALYZES".into(),
                from: analysis_run.id.clone(),
                to: change_set.id.clone(),
            },
            DerivedRelationship {
                kind: "CONTAINS_CHANGE".into(),
                from: change_set.id.clone(),
                to: derive_sha256_id(
                    "chie-change-record",
                    "change-set",
                    &change_record.change_set_id,
                ),
            },
            DerivedRelationship {
                kind: "PRODUCES".into(),
                from: analysis_run.id.clone(),
                to: impact_id.clone(),
            },
        ];
        if let Some(impact) = analysis.impact.as_ref() {
            relationships.extend(impact.impacted_entity_ids.iter().map(|entity_id| {
                DerivedRelationship {
                    kind: "AFFECTS".into(),
                    from: impact_id.clone(),
                    to: entity_id.clone(),
                }
            }));
        }
        if analysis.risk_assessment.is_some() {
            relationships.push(DerivedRelationship {
                kind: "HAS_RISK".into(),
                from: analysis_run.id.clone(),
                to: risk_id.clone(),
            });
            relationships.extend(analysis.evidence_records.iter().map(|evidence| {
                DerivedRelationship {
                    kind: "SUPPORTED_BY".into(),
                    from: risk_id.clone(),
                    to: evidence.id.clone(),
                }
            }));
        }
        relationships.extend(analysis.test_candidates.iter().map(|candidate| {
            DerivedRelationship {
                kind: "RECOMMENDS".into(),
                from: analysis_run.id.clone(),
                to: candidate.test_id.clone(),
            }
        }));
        relationships.extend(analysis.test_candidates.iter().flat_map(|candidate| {
            analysis
                .evidence_records
                .iter()
                .filter(|&evidence| evidence.test_id.as_deref() == Some(candidate.test_id.as_str()))
                .map(|evidence| DerivedRelationship {
                    kind: "SUPPORTED_BY".into(),
                    from: candidate.test_id.clone(),
                    to: evidence.id.clone(),
                })
        }));
        relationships.extend(analysis.evidence_records.iter().map(|evidence| {
            DerivedRelationship {
                kind: "SUPPORTED_BY".into(),
                from: impact_id.clone(),
                to: evidence.id.clone(),
            }
        }));
        relationships.sort_by(|a, b| (&a.kind, &a.from, &a.to).cmp(&(&b.kind, &b.from, &b.to)));
        relationships.dedup_by(|a, b| a.kind == b.kind && a.from == b.from && a.to == b.to);

        let report = self
            .storage
            .verify_change_analysis(&ChangeGraphVerificationExpectation {
                context: analysis.context.clone(),
                analysis_run_id: analysis_run.id.clone(),
                expected_entity_ids: entity_ids,
                expected_relationships: relationships,
            })
            .await?;
        if !report.semantic_match {
            return Err(PipelineError::Storage(StorageError::Query(
                "Change Graph read-after-write verification failed".to_owned(),
            )));
        }
        Ok(())
    }

    async fn stage_start(
        &self,
        operation: &mut Operation,
        stage: &str,
    ) -> Result<(), PipelineError> {
        self.stage_start_with_message(operation, stage, "stage started")
            .await
    }

    async fn stage_start_with_message(
        &self,
        operation: &mut Operation,
        stage: &str,
        message: &str,
    ) -> Result<(), PipelineError> {
        let mut event = operation
            .begin_stage(stage)
            .map_err(PipelineError::Engine)?;
        event.message = Some(message.to_owned());
        self.storage.persist_operation(&operation.record).await?;
        self.persist_operation_event(&event).await?;
        tracing::info!(operation_id = %operation.record.operation_id, stage, message, "engine stage started");
        Ok(())
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
        self.persist_operation_event(&event).await?;
        tracing::info!(operation_id = %operation.record.operation_id, stage, "engine stage completed");
        Ok(())
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

fn validate_repository_context(context: &RepositoryContext) -> Result<(), PipelineError> {
    context
        .validate()
        .map_err(|error| PipelineError::InvalidInput(error.to_string()))
}

fn validate_search_query(query: &str) -> Result<&str, PipelineError> {
    let query = query.trim();
    if query.is_empty() || query.len() > 500 {
        return Err(PipelineError::InvalidInput(
            "query must contain 1 to 500 bytes".to_owned(),
        ));
    }
    Ok(query)
}

fn validate_symbol_id(symbol_id: &str) -> Result<(), PipelineError> {
    let symbol_id = symbol_id.trim();
    if symbol_id.is_empty() || symbol_id.len() > 512 || symbol_id.chars().any(char::is_control) {
        return Err(PipelineError::InvalidInput(
            "symbol_id must contain 1 to 512 bytes and no control characters".to_owned(),
        ));
    }
    Ok(())
}

fn validate_change_depth(max_depth: usize) -> Result<(), PipelineError> {
    if !(1..=8).contains(&max_depth) {
        return Err(PipelineError::InvalidInput(
            "max_depth must be between 1 and 8".to_owned(),
        ));
    }
    Ok(())
}

fn bounded_query_limit(requested_limit: usize) -> usize {
    requested_limit.clamp(1, 100)
}

fn select_entity_id<'a>(
    candidates: impl IntoIterator<Item = (&'a str, &'a str, bool)>,
    selected_name: &str,
) -> Result<&'a str, PipelineError> {
    if selected_name.trim().is_empty() {
        return Err(PipelineError::InvalidInput(
            "symbol_name is required".to_owned(),
        ));
    }

    let mut matches = candidates
        .into_iter()
        .filter(|(_, name, selectable)| *selectable && *name == selected_name)
        .map(|(id, _, _)| id);
    let selected = matches
        .next()
        .ok_or(PipelineError::SelectedEntityNotFound)?;
    if matches.next().is_some() {
        return Err(PipelineError::SelectedEntityAmbiguous);
    }
    Ok(selected)
}

pub(crate) fn investigation_stage_order() -> [&'static str; 1] {
    ["chie"]
}

/// Repository-level decision made before source capture and expensive graph
/// analysis. The decision is intentionally independent of the question: the
/// repository either has reusable knowledge, needs initialization, or needs an
/// update. Incremental mutation is still gated separately below.
fn repository_knowledge_requires_partial(
    knowledge_is_complete: bool,
    omitted_file_count: u64,
    history_complete: bool,
    omitted_commits_at_least: u64,
    omitted_path_changes_at_least: u64,
) -> bool {
    !knowledge_is_complete
        || omitted_file_count > 0
        || !history_complete
        || omitted_commits_at_least > 0
        || omitted_path_changes_at_least > 0
}

fn repository_completeness_claim_is_consistent(
    omitted_file_count: u64,
    history_complete: bool,
    omitted_commits_at_least: u64,
    omitted_path_changes_at_least: u64,
) -> bool {
    omitted_file_count == 0
        && history_complete
        && omitted_commits_at_least == 0
        && omitted_path_changes_at_least == 0
}

fn mark_analysis_partial(analysis: &mut crate::engines::chie::model::ChangeAnalysis) {
    analysis.truncated = true;
    if let Some(impact) = analysis.impact.as_mut() {
        impact.truncated = true;
        impact.availability = crate::engines::chie::model::EvidenceAvailability::Partial;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepositoryPipelineDecision {
    Reuse,
    Initialize,
    Update,
}

/// Decide whether the repository index pipeline may be skipped, must build
/// from scratch, or must refresh an existing repository graph. A missing
/// current commit is conservative: known graph state becomes `Update` rather
/// than being incorrectly treated as current.
pub(crate) fn decide_repository_pipeline(
    repository_known: bool,
    graph_usable: bool,
    last_verified_commit: Option<&str>,
    current_commit: Option<&str>,
) -> RepositoryPipelineDecision {
    if !repository_known || !graph_usable {
        return RepositoryPipelineDecision::Initialize;
    }

    match (last_verified_commit, current_commit) {
        (Some(verified), Some(current)) if verified.eq_ignore_ascii_case(current) => {
            RepositoryPipelineDecision::Reuse
        }
        _ => RepositoryPipelineDecision::Update,
    }
}

/// Repository update strategy after the Git diff boundary has produced a
/// complete change set. Incomplete or unavailable evidence always falls back
/// to the correctness-first full rebuild path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepositoryUpdateStrategy {
    IncrementalMutation,
    FullRebuild,
}

pub(crate) fn select_repository_update_strategy(
    diff_complete: Option<bool>,
) -> RepositoryUpdateStrategy {
    match diff_complete {
        Some(true) => RepositoryUpdateStrategy::IncrementalMutation,
        Some(false) | None => RepositoryUpdateStrategy::FullRebuild,
    }
}

/// Path-level incremental mutation requires a lossless mapping from Git paths
/// into the source snapshot. Non-UTF-8 Git paths cannot currently be addressed
/// by the UTF-8 source manifest, so they conservatively force a full rebuild.
pub(crate) fn incremental_path_mapping_is_safe(diff: &crate::git::GitDiff) -> bool {
    diff.complete
        && !diff.changes.is_empty()
        && diff.changes.iter().all(|change| {
            change
                .old_path
                .as_ref()
                .and_then(|path| path.raw_bytes_hex.as_ref())
                .is_none()
                && change
                    .new_path
                    .as_ref()
                    .and_then(|path| path.raw_bytes_hex.as_ref())
                    .is_none()
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn repository_knowledge_is_partial_only_for_incomplete_or_omitted_inputs() {
        assert!(!super::repository_knowledge_requires_partial(
            true, 0, true, 0, 0
        ));
        assert!(super::repository_knowledge_requires_partial(
            false, 0, true, 0, 0
        ));
        assert!(super::repository_knowledge_requires_partial(
            true, 1, true, 0, 0
        ));
        assert!(super::repository_knowledge_requires_partial(
            true, 0, false, 0, 0
        ));
        assert!(super::repository_knowledge_requires_partial(
            true, 0, true, 1, 0
        ));
        assert!(super::repository_knowledge_requires_partial(
            true, 0, true, 0, 1
        ));
        // Freshness is carried independently. A complete pinned snapshot can
        // be stale or unknown without becoming truncated for that snapshot.
    }

    #[test]
    fn repository_completeness_is_independent_from_source_freshness() {
        assert!(super::repository_completeness_claim_is_consistent(
            0, true, 0, 0
        ));
        assert!(!super::repository_completeness_claim_is_consistent(
            1, true, 0, 0
        ));
        assert!(!super::repository_completeness_claim_is_consistent(
            0, false, 0, 0
        ));
        assert!(!super::repository_completeness_claim_is_consistent(
            0, true, 1, 0
        ));
        assert!(!super::repository_completeness_claim_is_consistent(
            0, true, 0, 1
        ));
        // Freshness is deliberately absent from this predicate. A graph may
        // be complete for its pinned snapshot while the remote/local source has
        // moved or could not be revalidated; the investigation result reports
        // those facts on separate axes.
    }

    #[test]
    fn verified_pipeline_contains_only_question_specific_stage() {
        assert_eq!(super::investigation_stage_order(), ["chie"]);
    }

    #[test]
    fn intelligence_query_inputs_are_trimmed_and_bounded() {
        assert_eq!(
            super::validate_search_query("  parser  ").unwrap(),
            "parser"
        );
        assert!(super::validate_search_query("  ").is_err());
        assert!(super::validate_search_query(&"x".repeat(501)).is_err());
        assert_eq!(super::bounded_query_limit(0), 1);
        assert_eq!(super::bounded_query_limit(50), 50);
        assert_eq!(super::bounded_query_limit(500), 100);
        assert!(super::validate_symbol_id(" ").is_err());
        assert!(super::validate_symbol_id("symbol-1").is_ok());
        assert!(super::validate_symbol_id(&"x".repeat(513)).is_err());
        assert!(super::validate_symbol_id("symbol\n1").is_err());
    }

    #[test]
    fn stored_change_query_depth_is_explicitly_bounded() {
        assert!(super::validate_change_depth(1).is_ok());
        assert!(super::validate_change_depth(8).is_ok());
        assert!(super::validate_change_depth(0).is_err());
        assert!(super::validate_change_depth(9).is_err());
    }

    #[test]
    fn prepared_selection_requires_one_exact_supported_entity() {
        let candidates = [
            ("file-1", "main.rs", true),
            ("other-1", "target", false),
            ("symbol-1", "target", true),
        ];
        assert_eq!(
            super::select_entity_id(candidates, "target").unwrap(),
            "symbol-1"
        );
        assert!(matches!(
            super::select_entity_id(
                [("id-1", "target", true), ("id-2", "target", true)],
                "target"
            ),
            Err(super::PipelineError::SelectedEntityAmbiguous)
        ));
        assert!(matches!(
            super::select_entity_id([("id-1", "other", true)], "target"),
            Err(super::PipelineError::SelectedEntityNotFound)
        ));
        assert!(matches!(
            super::select_entity_id([("id-1", "target", true)], "  "),
            Err(super::PipelineError::InvalidInput(_))
        ));
    }

    #[test]
    fn phase9_pipeline_gate_reuses_only_matching_verified_commits() {
        assert_eq!(
            super::decide_repository_pipeline(true, true, Some("abc"), Some("ABC")),
            super::RepositoryPipelineDecision::Reuse
        );
        assert_eq!(
            super::decide_repository_pipeline(true, true, Some("abc"), Some("def")),
            super::RepositoryPipelineDecision::Update
        );
    }

    #[test]
    fn phase9_pipeline_gate_initializes_when_repository_or_graph_is_unusable() {
        assert_eq!(
            super::decide_repository_pipeline(false, false, None, None),
            super::RepositoryPipelineDecision::Initialize
        );
        assert_eq!(
            super::decide_repository_pipeline(true, false, Some("abc"), Some("abc")),
            super::RepositoryPipelineDecision::Initialize
        );
    }

    #[test]
    fn phase9_pipeline_gate_updates_when_current_commit_is_unknown() {
        assert_eq!(
            super::decide_repository_pipeline(true, true, Some("abc"), None),
            super::RepositoryPipelineDecision::Update
        );
        assert_eq!(
            super::decide_repository_pipeline(true, true, None, Some("abc")),
            super::RepositoryPipelineDecision::Update
        );
    }

    #[test]
    fn phase15_selects_incremental_mutation_only_for_complete_diff() {
        assert_eq!(
            super::select_repository_update_strategy(Some(true)),
            super::RepositoryUpdateStrategy::IncrementalMutation
        );
        assert_eq!(
            super::select_repository_update_strategy(Some(false)),
            super::RepositoryUpdateStrategy::FullRebuild
        );
        assert_eq!(
            super::select_repository_update_strategy(None),
            super::RepositoryUpdateStrategy::FullRebuild
        );
    }
}

#[cfg(test)]
mod prepared_selection_tests {
    use super::{PipelineRequest, PreparedSelection};
    use crate::{engines::coie::CoieEngine, source::SourceProvider};
    use std::fs;

    #[test]
    fn pipeline_request_requires_the_same_prepared_graph_used_for_selection() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("lib.rs"), "pub fn selected() {}\n").unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "prepared-selection")
            .unwrap();
        let graph = CoieEngine.extract(&snapshot).unwrap();
        let selected = graph
            .entities
            .iter()
            .find(|entity| entity.name == "selected")
            .expect("fixture symbol must be extracted")
            .id
            .clone();
        let selection = PreparedSelection {
            context: snapshot.context.clone(),
            graph,
            selected_entity_id: selected.clone(),
        };
        let request = PipelineRequest {
            snapshot: snapshot.clone(),
            selected_entity_id: selected,
            request_id: "request".into(),
            trace_id: "trace".into(),
            operation_id: None,
            prepared_selection: Some(selection),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn pipeline_request_rejects_a_selection_from_another_extraction() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("lib.rs"), "pub fn selected() {}\n").unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "prepared-selection-mismatch")
            .unwrap();
        let graph = CoieEngine.extract(&snapshot).unwrap();
        let selected = graph
            .entities
            .iter()
            .find(|entity| entity.name == "selected")
            .expect("fixture symbol must be extracted")
            .id
            .clone();
        let selection = PreparedSelection {
            context: snapshot.context.clone(),
            graph,
            selected_entity_id: selected.clone(),
        };
        let request = PipelineRequest {
            snapshot,
            selected_entity_id: "id-from-another-extraction".into(),
            request_id: "request".into(),
            trace_id: "trace".into(),
            operation_id: None,
            prepared_selection: Some(selection),
        };

        assert!(request.validate().is_err());
    }
}

