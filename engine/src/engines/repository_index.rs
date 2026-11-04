//! Repository-level indexing pipeline.
//!
//! This pipeline builds durable repository knowledge once from one pinned
//! source snapshot and one Git history scan. Question-specific ChIE work stays
//! in `pipeline.rs` and can therefore run after an owned clone is removed.

use crate::{
    contract::events::EngineEvent,
    engines::{
        coie::CoieEngine,
        history::build_history_graph,
        tie::{TieEngine, graph_builder as tie_graph_builder, mapping::target_mapping},
    },
    graph_storage::{
        GraphStorage, GraphVerificationExpectation, GraphVerificationReport, StorageError,
        digest_identity_set,
    },
    lifecycle::Operation,
    repository::PreparedRepository,
};
use std::sync::Arc;
use thiserror::Error;

/// Incremental graph mutation is unavailable until the adapter can stage and
/// atomically publish complete Code/Test generations without modifying the
/// last verified generation in place.
pub const INCREMENTAL_MUTATION_PRODUCTION_ENABLED: bool = false;

#[derive(Debug, Clone)]
pub struct RepositoryIndexResult {
    pub verification: GraphVerificationReport,
    pub code_entity_count: usize,
    pub code_relationship_count: usize,
    pub test_entity_count: usize,
    pub test_relationship_count: usize,
    pub history_entity_count: usize,
    pub history_relationship_count: usize,
    pub complete: bool,
    pub omitted_file_count: u64,
    pub history_complete: bool,
    pub history_omitted_commits_at_least: u64,
    pub history_omitted_path_changes_at_least: u64,
    pub head_commit: String,
}

#[derive(Debug, Error)]
pub enum RepositoryIndexError {
    #[error("repository indexing engine error: {0}")]
    Engine(String),
    #[error("repository indexing storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("graph storage did not prove the exact repository index expectation")]
    VerificationMismatch,
}

/// Build and verify the durable repository graph slices from one prepared
/// repository. The source snapshot is immutable-by-convention and the Git
/// history is read before the owned workspace can be cleaned.
pub struct RepositoryIndexPipeline<S> {
    storage: Arc<S>,
    coie: CoieEngine,
    tie: TieEngine,
}

impl<S> RepositoryIndexPipeline<S>
where
    S: GraphStorage + 'static,
{
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            coie: CoieEngine,
            tie: TieEngine,
        }
    }

    pub async fn run(
        &self,
        prepared: &PreparedRepository,
        operation: &mut Operation,
    ) -> Result<RepositoryIndexResult, RepositoryIndexError> {
        let stages = stage_order();

        self.stage_start(operation, stages[0]).await?;
        let graph = self
            .coie
            .extract(&prepared.snapshot)
            .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        crate::contract::intelligence::CoieResult {
            graph: graph.clone(),
        }
        .validate(&prepared.snapshot.context)
        .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        self.storage.write_code_graph(&graph).await?;
        self.stage_finish(operation, stages[0]).await?;

        self.stage_start(operation, stages[1]).await?;
        let history_context = crate::engines::history::HistoryGraphContext::from_source_context(
            &prepared.snapshot.context,
        )
        .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        let history_graph = build_history_graph(history_context, &prepared.git_history)
            .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        history_graph
            .validate()
            .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        let generation_id =
            crate::graph_storage::generation_id_for_context(&prepared.snapshot.context);
        self.storage
            .write_history_graph(&history_graph, &generation_id)
            .await?;
        self.stage_finish(operation, stages[1]).await?;

        self.stage_start(operation, stages[2]).await?;
        let (tests, relationships) = self
            .tie
            .discover(&prepared.snapshot)
            .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        let (tests, relationships) = target_mapping::resolve_graph(&graph, tests, relationships);
        tie_graph_builder::validate_against_code_graph(&tests, &relationships, &graph)
            .map_err(RepositoryIndexError::Engine)?;
        crate::contract::intelligence::TieResult {
            tests: tests.clone(),
            relationships: relationships.clone(),
        }
        .validate(&prepared.snapshot.context)
        .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        self.storage
            .write_test_graph(&prepared.snapshot.context, &tests, &relationships)
            .await?;
        self.stage_finish(operation, stages[2]).await?;

        self.stage_start(operation, stages[3]).await?;
        let expected = GraphVerificationExpectation {
            context: prepared.snapshot.context.clone(),
            generation_id: crate::graph_storage::generation_id_for_context(
                &prepared.snapshot.context,
            ),
            code_entities: graph.entities.len(),
            code_relationships: graph.relationships.len(),
            code_entity_ids_digest: digest_identity_set(
                &graph
                    .entities
                    .iter()
                    .map(|entity| entity.id.clone())
                    .collect::<Vec<_>>(),
            ),
            code_relationship_ids_digest: digest_identity_set(
                &graph
                    .relationships
                    .iter()
                    .map(|relationship| relationship.id.clone())
                    .collect::<Vec<_>>(),
            ),
            test_entities: tests.len(),
            test_relationships: relationships
                .iter()
                .filter(|relationship| {
                    relationship.resolution == crate::engines::tie::model::TestResolution::Resolved
                })
                .count(),
            test_entity_ids_digest: digest_identity_set(
                &tests.iter().map(|test| test.id.clone()).collect::<Vec<_>>(),
            ),
            test_relationship_ids_digest: digest_identity_set(
                &relationships
                    .iter()
                    .filter(|relationship| {
                        relationship.resolution
                            == crate::engines::tie::model::TestResolution::Resolved
                    })
                    .map(|relationship| relationship.id.clone())
                    .collect::<Vec<_>>(),
            ),
            history_entities: history_graph.entities.len(),
            history_relationships: history_graph.relationships.len(),
            history_head_commit: history_graph.head_commit.clone(),
            history_complete: history_graph.complete,
            history_omitted_commits_at_least: history_graph.omitted_commits_at_least,
            history_omitted_path_changes_at_least: history_graph.omitted_path_changes_at_least,
            change_entities: 0,
            change_relationships: 0,
        };
        let verification = self
            .storage
            .verify_graph(&prepared.snapshot.context, &expected)
            .await?;
        ensure_verification_matches(&verification, &expected)?;
        self.stage_finish(operation, stages[3]).await?;

        let complete = repository_index_is_complete(
            prepared.snapshot.manifest.omitted_files.len() as u64,
            history_graph.complete,
            history_graph.omitted_commits_at_least,
            history_graph.omitted_path_changes_at_least,
        );
        Ok(RepositoryIndexResult {
            verification,
            code_entity_count: graph.entities.len(),
            code_relationship_count: graph.relationships.len(),
            test_entity_count: tests.len(),
            test_relationship_count: relationships
                .iter()
                .filter(|relationship| {
                    relationship.resolution == crate::engines::tie::model::TestResolution::Resolved
                })
                .count(),
            history_entity_count: history_graph.entities.len(),
            history_relationship_count: history_graph.relationships.len(),
            complete,
            omitted_file_count: prepared.snapshot.manifest.omitted_files.len() as u64,
            history_complete: history_graph.complete,
            history_omitted_commits_at_least: history_graph.omitted_commits_at_least,
            history_omitted_path_changes_at_least: history_graph.omitted_path_changes_at_least,
            head_commit: prepared.git_history.head_commit.clone(),
        })
    }

    pub async fn run_incremental(
        &self,
        previous_context: &crate::source::RepositoryContext,
        prepared: &PreparedRepository,
        diff: &crate::git::GitDiff,
        operation: &mut Operation,
    ) -> Result<RepositoryIndexResult, RepositoryIndexError> {
        if !INCREMENTAL_MUTATION_PRODUCTION_ENABLED {
            return Err(RepositoryIndexError::Engine(
                "incremental graph mutation is disabled until generation-staged publication and full-rebuild parity are accepted; use the full rebuild path".to_owned(),
            ));
        }
        if !diff.complete || diff.changes.is_empty() {
            return Err(RepositoryIndexError::Engine(
                "incremental indexing requires a complete non-empty Git diff".to_owned(),
            ));
        }

        let affected_paths = affected_paths(diff);
        let stages = incremental_stage_order();

        self.stage_start(operation, stages[0]).await?;
        let graph = self
            .coie
            .extract(&prepared.snapshot)
            .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        crate::contract::intelligence::CoieResult {
            graph: graph.clone(),
        }
        .validate(&prepared.snapshot.context)
        .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        self.storage
            .mutate_code_graph(previous_context, &graph, &affected_paths)
            .await?;
        self.stage_finish(operation, stages[0]).await?;

        self.stage_start(operation, stages[1]).await?;
        let history_context = crate::engines::history::HistoryGraphContext::from_source_context(
            &prepared.snapshot.context,
        )
        .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        let history_graph = build_history_graph(history_context, &prepared.git_history)
            .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        history_graph
            .validate()
            .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        let generation_id =
            crate::graph_storage::generation_id_for_context(&prepared.snapshot.context);
        self.storage
            .write_history_graph(&history_graph, &generation_id)
            .await?;
        self.stage_finish(operation, stages[1]).await?;

        self.stage_start(operation, stages[2]).await?;
        let (tests, relationships) = self
            .tie
            .discover(&prepared.snapshot)
            .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        let (tests, relationships) = target_mapping::resolve_graph(&graph, tests, relationships);
        tie_graph_builder::validate_against_code_graph(&tests, &relationships, &graph)
            .map_err(RepositoryIndexError::Engine)?;
        crate::contract::intelligence::TieResult {
            tests: tests.clone(),
            relationships: relationships.clone(),
        }
        .validate(&prepared.snapshot.context)
        .map_err(|error| RepositoryIndexError::Engine(error.to_string()))?;
        let affected_code_entity_ids: Vec<String> = graph
            .entities
            .iter()
            .filter(|entity| affected_paths.iter().any(|path| path == &entity.path))
            .map(|entity| entity.id.clone())
            .collect();
        self.storage
            .mutate_test_graph(
                previous_context,
                &prepared.snapshot.context,
                &tests,
                &relationships,
                &affected_paths,
                &affected_code_entity_ids,
            )
            .await?;
        self.stage_finish(operation, stages[2]).await?;

        self.stage_start(operation, stages[3]).await?;
        let expected = GraphVerificationExpectation {
            context: prepared.snapshot.context.clone(),
            generation_id: crate::graph_storage::generation_id_for_context(
                &prepared.snapshot.context,
            ),
            code_entities: graph.entities.len(),
            code_relationships: graph.relationships.len(),
            code_entity_ids_digest: digest_identity_set(
                &graph
                    .entities
                    .iter()
                    .map(|entity| entity.id.clone())
                    .collect::<Vec<_>>(),
            ),
            code_relationship_ids_digest: digest_identity_set(
                &graph
                    .relationships
                    .iter()
                    .map(|relationship| relationship.id.clone())
                    .collect::<Vec<_>>(),
            ),
            test_entities: tests.len(),
            test_relationships: relationships
                .iter()
                .filter(|relationship| {
                    relationship.resolution == crate::engines::tie::model::TestResolution::Resolved
                })
                .count(),
            test_entity_ids_digest: digest_identity_set(
                &tests.iter().map(|test| test.id.clone()).collect::<Vec<_>>(),
            ),
            test_relationship_ids_digest: digest_identity_set(
                &relationships
                    .iter()
                    .filter(|relationship| {
                        relationship.resolution
                            == crate::engines::tie::model::TestResolution::Resolved
                    })
                    .map(|relationship| relationship.id.clone())
                    .collect::<Vec<_>>(),
            ),
            history_entities: history_graph.entities.len(),
            history_relationships: history_graph.relationships.len(),
            history_head_commit: history_graph.head_commit.clone(),
            history_complete: history_graph.complete,
            history_omitted_commits_at_least: history_graph.omitted_commits_at_least,
            history_omitted_path_changes_at_least: history_graph.omitted_path_changes_at_least,
            change_entities: 0,
            change_relationships: 0,
        };
        let verification = self
            .storage
            .verify_graph(&prepared.snapshot.context, &expected)
            .await?;
        ensure_verification_matches(&verification, &expected)?;
        self.stage_finish(operation, stages[3]).await?;

        let complete = repository_index_is_complete(
            prepared.snapshot.manifest.omitted_files.len() as u64,
            history_graph.complete,
            history_graph.omitted_commits_at_least,
            history_graph.omitted_path_changes_at_least,
        );
        Ok(RepositoryIndexResult {
            verification,
            code_entity_count: graph.entities.len(),
            code_relationship_count: graph.relationships.len(),
            test_entity_count: tests.len(),
            test_relationship_count: relationships
                .iter()
                .filter(|relationship| {
                    relationship.resolution == crate::engines::tie::model::TestResolution::Resolved
                })
                .count(),
            history_entity_count: history_graph.entities.len(),
            history_relationship_count: history_graph.relationships.len(),
            complete,
            omitted_file_count: prepared.snapshot.manifest.omitted_files.len() as u64,
            history_complete: history_graph.complete,
            history_omitted_commits_at_least: history_graph.omitted_commits_at_least,
            history_omitted_path_changes_at_least: history_graph.omitted_path_changes_at_least,
            head_commit: prepared.git_history.head_commit.clone(),
        })
    }

    async fn stage_start(
        &self,
        operation: &mut Operation,
        stage: &str,
    ) -> Result<(), RepositoryIndexError> {
        let event = operation
            .begin_stage(stage)
            .map_err(RepositoryIndexError::Engine)?;
        self.storage.persist_operation(&operation.record).await?;
        self.persist_event(&event).await
    }

    async fn stage_finish(
        &self,
        operation: &mut Operation,
        stage: &str,
    ) -> Result<(), RepositoryIndexError> {
        let event = operation
            .finish_stage(stage)
            .map_err(RepositoryIndexError::Engine)?;
        self.storage.persist_operation(&operation.record).await?;
        self.persist_event(&event).await
    }

    async fn persist_event(
        &self,
        event: &crate::lifecycle::operation::OperationEvent,
    ) -> Result<(), RepositoryIndexError> {
        self.storage
            .append_operation_event(&EngineEvent::Progress {
                metadata: event.metadata.clone(),
                engine: "codemind".to_owned(),
                stage: event
                    .stage
                    .clone()
                    .unwrap_or_else(|| "repository-index".to_owned()),
                message: event.message.clone().unwrap_or_default(),
            })
            .await?;
        Ok(())
    }
}

fn ensure_verification_matches(
    report: &GraphVerificationReport,
    expected: &GraphVerificationExpectation,
) -> Result<(), RepositoryIndexError> {
    if report.satisfies(expected) {
        Ok(())
    } else {
        Err(RepositoryIndexError::VerificationMismatch)
    }
}

fn repository_index_is_complete(
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

/// The repository indexer deliberately has no ChIE stage. This pure value is
/// used by module-level tests to keep the indexing order explicit.
fn affected_paths(diff: &crate::git::GitDiff) -> Vec<String> {
    let mut paths = std::collections::BTreeSet::new();
    for change in &diff.changes {
        for path in [change.old_path.as_ref(), change.new_path.as_ref()]
            .into_iter()
            .flatten()
        {
            let mut current = path.display.clone();
            loop {
                if current.is_empty() {
                    break;
                }
                paths.insert(current.clone());
                let Some(parent) = std::path::Path::new(&current).parent() else {
                    break;
                };
                let parent = parent.to_string_lossy().replace('\\', "/");
                if parent.is_empty() || parent == current {
                    break;
                }
                current = parent;
            }
        }
    }
    paths.into_iter().collect()
}

pub(crate) fn incremental_stage_order() -> [&'static str; 4] {
    [
        "repository-update.incremental.coie",
        "repository-update.incremental.history",
        "repository-update.incremental.tie",
        "repository-update.incremental.verify",
    ]
}

pub(crate) fn stage_order() -> [&'static str; 4] {
    [
        "repository-index.coie",
        "repository-index.history",
        "repository-index.tie",
        "repository-index.verify",
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn repository_index_stage_order_is_stable() {
        assert_eq!(
            super::stage_order(),
            [
                "repository-index.coie",
                "repository-index.history",
                "repository-index.tie",
                "repository-index.verify",
            ]
        );
    }

    #[test]
    fn repository_index_has_no_question_specific_stage() {
        assert!(
            super::stage_order()
                .iter()
                .all(|stage| !stage.contains("chie"))
        );
    }

    #[test]
    fn source_or_history_omissions_keep_repository_index_partial() {
        assert!(super::repository_index_is_complete(0, true, 0, 0));
        assert!(!super::repository_index_is_complete(1, true, 0, 0));
        assert!(!super::repository_index_is_complete(0, false, 0, 0));
        assert!(!super::repository_index_is_complete(0, true, 1, 0));
        assert!(!super::repository_index_is_complete(0, true, 0, 1));
    }

    #[test]
    fn incremental_stage_order_is_explicit_and_has_no_chie_stage() {
        let stages = super::incremental_stage_order();
        assert_eq!(stages[0], "repository-update.incremental.coie");
        assert_eq!(stages[3], "repository-update.incremental.verify");
        assert!(stages.iter().all(|stage| !stage.contains("chie")));
    }
}
