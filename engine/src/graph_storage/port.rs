use crate::contract::events::EngineEvent;
use crate::contract::operation::OperationRecord;
use crate::engines::chie::{AnalysisRun, ChangeAnalysis, ChangeRecord, ChangeSet};
use crate::engines::coie::model::{CodeGraph, GraphEntity, GraphRelationship};
use crate::engines::history::model::{HistoryEntity, HistoryGraph};
use crate::engines::tie::model::{TestEntity, TestRelationship};
use crate::{source::RepositoryContext, source::identity::digest_text};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphPath {
    pub terminal_entity_id: String,
    pub relationships: Vec<GraphRelationship>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyPathResult {
    pub paths: Vec<GraphPath>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTestGraph {
    pub tests: Vec<TestEntity>,
    pub relationships: Vec<TestRelationship>,
    /// True when either bounded storage query had more rows than it retained.
    pub truncated: bool,
}

/// Result of repository-scoped Code/Test projection retention. Candidates are
/// computed only after the requested generation has been proven active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRetentionReport {
    pub workspace_id: String,
    pub repository_id: String,
    pub active_snapshot_id: String,
    pub active_graph_version: String,
    pub eligible_code_entities: usize,
    pub eligible_test_entities: usize,
    pub deleted_code_entities: usize,
    pub deleted_test_entities: usize,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedRelationship {
    pub kind: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphVerificationExpectation {
    pub context: RepositoryContext,
    /// Publication generation that the repository lifecycle is preparing or
    /// has already published. History Graph verification uses this to keep a
    /// partially written history update isolated from the last verified one.
    pub generation_id: String,
    pub code_entities: usize,
    pub code_relationships: usize,
    pub code_entity_ids_digest: String,
    pub code_relationship_ids_digest: String,
    pub test_entities: usize,
    pub test_relationships: usize,
    pub test_entity_ids_digest: String,
    pub test_relationship_ids_digest: String,
    pub history_entities: usize,
    pub history_relationships: usize,
    pub history_head_commit: String,
    pub history_complete: bool,
    pub history_omitted_commits_at_least: u64,
    pub history_omitted_path_changes_at_least: u64,
    /// Change Graph is historical and may contain multiple analysis runs.
    /// Repository-index verification does not require a Change Graph.
    pub change_entities: usize,
    pub change_relationships: usize,
}

impl GraphVerificationExpectation {
    /// Validate the acceptance contract before a storage adapter performs any
    /// read-after-write verification. The expected context must be explicit
    /// and the History Graph must identify a concrete Git head.
    pub fn validate(&self) -> Result<(), StorageError> {
        self.context
            .validate()
            .map_err(|_| StorageError::InvalidContext)?;
        if self.generation_id != generation_id_for_context(&self.context) {
            return Err(StorageError::Query(
                "graph verification generation does not match the repository context".to_owned(),
            ));
        }
        if !matches!(self.history_head_commit.len(), 40 | 64)
            || !self
                .history_head_commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(StorageError::Query(
                "graph verification requires a valid 40- or 64-character History Graph head commit"
                    .to_owned(),
            ));
        }
        if self.history_complete
            && (self.history_omitted_commits_at_least != 0
                || self.history_omitted_path_changes_at_least != 0)
        {
            return Err(StorageError::Query(
                "complete History Graph cannot report omitted commits or path changes".to_owned(),
            ));
        }
        for digest in [
            &self.code_entity_ids_digest,
            &self.code_relationship_ids_digest,
            &self.test_entity_ids_digest,
            &self.test_relationship_ids_digest,
        ] {
            if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(StorageError::Query(
                    "graph verification requires SHA-256 identity digests".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphVerificationReport {
    pub context: RepositoryContext,
    pub generation_id: String,
    pub code_entities: usize,
    pub code_relationships: usize,
    /// Deterministic digests of the Code Graph identities read back from storage.
    pub code_entity_ids_digest: String,
    pub code_relationship_ids_digest: String,
    pub test_entities: usize,
    pub test_relationships: usize,
    /// Deterministic digests of the Test Graph identities read back from storage.
    pub test_entity_ids_digest: String,
    pub test_relationship_ids_digest: String,
    pub history_entities: usize,
    pub history_relationships: usize,
    pub history_head_commit: String,
    pub history_complete: bool,
    pub history_omitted_commits_at_least: u64,
    pub history_omitted_path_changes_at_least: u64,
    pub change_entities: usize,
    pub change_relationships: usize,
    /// True only when the storage adapter proved the expected IDs and scope,
    /// not merely matching aggregate counts.
    pub semantic_match: bool,
}

impl GraphVerificationReport {
    pub fn satisfies(&self, expected: &GraphVerificationExpectation) -> bool {
        expected.validate().is_ok()
            && self.context == expected.context
            && self.generation_id == expected.generation_id
            && self.code_entities == expected.code_entities
            && self.code_relationships == expected.code_relationships
            && self.code_entity_ids_digest == expected.code_entity_ids_digest
            && self.code_relationship_ids_digest == expected.code_relationship_ids_digest
            && self.test_entities == expected.test_entities
            && self.test_relationships == expected.test_relationships
            && self.test_entity_ids_digest == expected.test_entity_ids_digest
            && self.test_relationship_ids_digest == expected.test_relationship_ids_digest
            && self.history_entities == expected.history_entities
            && self.history_relationships == expected.history_relationships
            && self
                .history_head_commit
                .eq_ignore_ascii_case(&expected.history_head_commit)
            && self.history_complete == expected.history_complete
            && self.history_omitted_commits_at_least == expected.history_omitted_commits_at_least
            && self.history_omitted_path_changes_at_least
                == expected.history_omitted_path_changes_at_least
            && self.change_entities >= expected.change_entities
            && self.change_relationships >= expected.change_relationships
            && self.semantic_match
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeGraphVerificationExpectation {
    pub context: RepositoryContext,
    pub analysis_run_id: String,
    pub expected_entity_ids: Vec<String>,
    pub expected_relationships: Vec<DerivedRelationship>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeGraphVerificationReport {
    pub context: RepositoryContext,
    pub analysis_run_id: String,
    pub entity_count: usize,
    pub relationship_count: usize,
    pub semantic_match: bool,
}

/// Database-independent failure surface for every graph and memory adapter.
///
/// Implementations must return an error when execution or result decoding fails.
/// An empty collection, `None`, or a zero count is reserved for a successful
/// query that actually found no matching data.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("graph storage unavailable: {0}")]
    Unavailable(String),
    #[error("graph query failed: {0}")]
    Query(String),
    #[error("graph storage mapping failed: {0}")]
    Mapping(String),
    #[error("incompatible graph storage schema: expected {expected}, found {actual}")]
    SchemaIncompatible { expected: u32, actual: u32 },
    #[error("invalid graph context")]
    InvalidContext,
}

/// Semantic storage port consumed by the engine and service layers.
///
/// The contract intentionally exposes CodeMind domain values and storage errors,
/// never a database client, physical graph handle, vendor query, or transaction.
/// `RepositoryContext.workspace_id` is the canonical physical isolation scope.
/// An adapter may map that validated scope to a physical partition, but must never
/// route by operation, conversation, snapshot, repository display name, or a graph
/// name supplied by a request. Adapter implementations are also responsible for
/// preserving the remaining repository/snapshot context, bounded reads, idempotency,
/// and explicit failure semantics.
#[async_trait]
pub trait GraphStorage: Send + Sync + crate::memory::repository::MemoryRepository {
    async fn persist_operation(
        &self,
        operation: &OperationRecord,
    ) -> Result<OperationRecord, StorageError>;

    async fn append_operation_event(&self, event: &EngineEvent) -> Result<(), StorageError>;

    async fn retrieve_operation(
        &self,
        context: &RepositoryContext,
        operation_id: &str,
    ) -> Result<OperationRecord, StorageError>;

    async fn events_since(
        &self,
        context: &RepositoryContext,
        operation_id: &str,
        after_sequence: u64,
        limit: usize,
    ) -> Result<Vec<EngineEvent>, StorageError>;

    async fn write_code_graph(&self, graph: &CodeGraph) -> Result<(), StorageError>;

    /// Apply a bounded commit-to-commit Code Graph update. Only entities and
    /// relationships in `affected_paths` are replaced; unrelated repository
    /// facts remain in place and are advanced to the new snapshot context.
    async fn mutate_code_graph(
        &self,
        previous_context: &RepositoryContext,
        graph: &CodeGraph,
        affected_paths: &[String],
    ) -> Result<(), StorageError>;

    /// Apply the Test Graph portion of an incremental update. Test facts in
    /// affected paths and relationships targeting affected code entities are
    /// replaced so stale mappings cannot survive a file change.
    async fn mutate_test_graph(
        &self,
        previous_context: &RepositoryContext,
        context: &RepositoryContext,
        tests: &[TestEntity],
        relationships: &[TestRelationship],
        affected_paths: &[String],
        affected_code_entity_ids: &[String],
    ) -> Result<(), StorageError>;

    async fn write_history_graph(
        &self,
        graph: &HistoryGraph,
        generation_id: &str,
    ) -> Result<(), StorageError>;

    async fn history_entities_for(
        &self,
        context: &crate::engines::history::model::HistoryGraphContext,
        limit: usize,
    ) -> Result<Vec<HistoryEntity>, StorageError>;

    async fn verify_graph(
        &self,
        context: &RepositoryContext,
        expected: &GraphVerificationExpectation,
    ) -> Result<GraphVerificationReport, StorageError>;

    async fn verified_snapshot_exists(
        &self,
        context: &RepositoryContext,
    ) -> Result<bool, StorageError>;

    /// Publish a fully verified source generation as the storage-side publication
    /// record. This marker is not a replacement for durable repository state;
    /// it prevents an unverified generation from being mistaken for a published
    /// one during reuse checks.
    async fn publish_verified_generation(
        &self,
        context: &RepositoryContext,
        generation_id: &str,
    ) -> Result<(), StorageError>;

    /// Remove obsolete Code/Test snapshot projections after the replacement
    /// generation is verified and published. Change Graph-referenced facts and
    /// all History Graph facts must be retained. Adapters without this
    /// capability fail explicitly rather than silently claiming retention.
    async fn prune_obsolete_graph_projections(
        &self,
        context: &RepositoryContext,
        generation_id: &str,
        dry_run: bool,
    ) -> Result<GraphRetentionReport, StorageError> {
        let _ = (context, generation_id, dry_run);
        Err(StorageError::Unavailable(
            "graph projection retention is not supported by this storage adapter".to_owned(),
        ))
    }

    async fn verify_published_generation(
        &self,
        context: &RepositoryContext,
        generation_id: &str,
    ) -> Result<bool, StorageError>;

    async fn verify_change_analysis(
        &self,
        expected: &ChangeGraphVerificationExpectation,
    ) -> Result<ChangeGraphVerificationReport, StorageError>;

    async fn write_test_graph(
        &self,
        context: &RepositoryContext,
        tests: &[TestEntity],
        relationships: &[TestRelationship],
    ) -> Result<(), StorageError>;

    /// Publish the derived ChIE result as a versioned Change Graph without mutating CoIE/TIE facts.
    async fn persist_change_analysis(
        &self,
        change_set: &ChangeSet,
        change_record: &ChangeRecord,
        analysis_run: &AnalysisRun,
        analysis: &ChangeAnalysis,
    ) -> Result<(), StorageError>;

    async fn derived_relationships_for_run(
        &self,
        context: &RepositoryContext,
        analysis_run_id: &str,
        limit: usize,
    ) -> Result<Vec<DerivedRelationship>, StorageError>;

    async fn entity_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
    ) -> Result<Option<GraphEntity>, StorageError>;

    async fn search_code_entities(
        &self,
        context: &RepositoryContext,
        query: &str,
        limit: usize,
    ) -> Result<Vec<GraphEntity>, StorageError>;

    /// Return every Symbol/File whose name exactly matches the supplied name.
    /// Unlike substring search this query is intended for uniqueness checks.
    async fn exact_code_entities_by_name(
        &self,
        context: &RepositoryContext,
        name: &str,
    ) -> Result<Vec<GraphEntity>, StorageError>;

    async fn relationships_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationship>, StorageError>;

    async fn incoming_relationships_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationship>, StorageError>;

    async fn dependency_incoming_relationships_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationship>, StorageError>;

    /// Return bounded dependency paths without loading the complete graph.
    /// Direction is represented by the stored edge itself; callers decide
    /// which relationship semantics constitute change propagation.
    async fn dependency_paths_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        max_depth: usize,
        max_paths: usize,
    ) -> Result<DependencyPathResult, StorageError>;

    async fn tests_for_context(
        &self,
        context: &RepositoryContext,
        target_entity_ids: &[String],
        limit: usize,
    ) -> Result<StoredTestGraph, StorageError>;
}

/// Derive the deterministic publication generation for one repository graph
/// projection. This is a storage-independent identity used by the repository
/// lifecycle and by the adapter's staged History Graph writes.
pub fn generation_id_for_context(context: &RepositoryContext) -> String {
    format!(
        "generation-{}",
        digest_text(&format!(
            "codemind-generation-v1\0{}\0{}\0{}\0{}",
            context.repository_id,
            context.workspace_id,
            context.source_snapshot_id,
            context.graph_version,
        ))
    )
}

/// Compute a deterministic digest for a set of stable graph identities.
/// Sorting makes the proof independent of database return order.
pub fn digest_identity_set(ids: &[String]) -> String {
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    let mut material = String::new();
    for id in sorted {
        material.push_str(&id.len().to_string());
        material.push(':');
        material.push_str(&id);
        material.push('|');
    }
    digest_text(&material)
}

#[cfg(test)]
mod level1_tests {
    use super::{StorageError, digest_identity_set};

    #[test]
    fn identity_digest_is_order_independent_and_preserves_duplicate_ids() {
        let a = digest_identity_set(&["b".into(), "a".into(), "a".into()]);
        let b = digest_identity_set(&["a".into(), "b".into()]);
        assert_ne!(a, b);
        assert_eq!(b, digest_identity_set(&["b".into(), "a".into()]));
    }

    #[test]
    fn storage_failures_have_database_independent_error_categories() {
        let unavailable = StorageError::Unavailable("connection interrupted".into());
        let query = StorageError::Query("query execution failed".into());
        let mapping = StorageError::Mapping("unknown persisted relationship label".into());
        let invalid_context = StorageError::InvalidContext;

        assert!(
            unavailable
                .to_string()
                .contains("graph storage unavailable")
        );
        assert!(query.to_string().contains("graph query failed"));
        assert!(mapping.to_string().contains("graph storage mapping failed"));
        assert!(
            invalid_context
                .to_string()
                .contains("invalid graph context")
        );
        assert_ne!(unavailable.to_string(), query.to_string());
        assert_ne!(query.to_string(), mapping.to_string());
    }
}

#[cfg(test)]
mod cross_module_tests {
    use super::*;

    #[test]
    fn verification_expectation_rejects_invalid_head_and_digest_contracts() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let mut expected = GraphVerificationExpectation {
            context: context.clone(),
            generation_id: generation_id_for_context(&context),
            code_entities: 0,
            code_relationships: 0,
            code_entity_ids_digest: "a".repeat(64),
            code_relationship_ids_digest: "b".repeat(64),
            test_entities: 0,
            test_relationships: 0,
            test_entity_ids_digest: "c".repeat(64),
            test_relationship_ids_digest: "d".repeat(64),
            history_entities: 0,
            history_relationships: 0,
            history_head_commit: "not-a-commit".into(),
            history_complete: false,
            history_omitted_commits_at_least: 1,
            history_omitted_path_changes_at_least: 1,
            change_entities: 0,
            change_relationships: 0,
        };
        assert!(expected.validate().is_err());
        expected.history_head_commit = "a".repeat(40);
        assert!(expected.validate().is_ok());
        expected.code_entity_ids_digest = "not-a-digest".into();
        assert!(expected.validate().is_err());
    }

    #[test]
    fn verification_report_requires_matching_semantics_and_stronger_counts() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let report = GraphVerificationReport {
            context: context.clone(),
            generation_id: generation_id_for_context(&context),
            code_entities: 2,
            code_relationships: 1,
            code_entity_ids_digest: digest_identity_set(&["code-a".into(), "code-b".into()]),
            code_relationship_ids_digest: digest_identity_set(&["code-rel".into()]),
            test_entities: 1,
            test_relationships: 1,
            test_entity_ids_digest: digest_identity_set(&["test-a".into()]),
            test_relationship_ids_digest: digest_identity_set(&["test-rel".into()]),
            history_entities: 3,
            history_relationships: 2,
            history_head_commit: "a".repeat(40),
            history_complete: true,
            history_omitted_commits_at_least: 0,
            history_omitted_path_changes_at_least: 0,
            change_entities: 2,
            change_relationships: 1,
            semantic_match: true,
        };
        let expected = GraphVerificationExpectation {
            context: report.context.clone(),
            generation_id: generation_id_for_context(&report.context),
            code_entities: 2,
            code_relationships: 1,
            code_entity_ids_digest: digest_identity_set(&["code-a".into(), "code-b".into()]),
            code_relationship_ids_digest: digest_identity_set(&["code-rel".into()]),
            test_entities: 1,
            test_relationships: 1,
            test_entity_ids_digest: digest_identity_set(&["test-a".into()]),
            test_relationship_ids_digest: digest_identity_set(&["test-rel".into()]),
            history_entities: 3,
            history_relationships: 2,
            history_head_commit: "a".repeat(40),
            history_complete: true,
            history_omitted_commits_at_least: 0,
            history_omitted_path_changes_at_least: 0,
            change_entities: 2,
            change_relationships: 1,
        };
        assert!(report.satisfies(&expected));
        assert!(
            !GraphVerificationReport {
                code_entity_ids_digest: digest_identity_set(&["different-code-id".into()]),
                ..report.clone()
            }
            .satisfies(&expected)
        );
        assert!(
            !GraphVerificationReport {
                code_relationship_ids_digest: digest_identity_set(&[
                    "different-code-relationship".into()
                ]),
                ..report.clone()
            }
            .satisfies(&expected)
        );
        assert!(
            !GraphVerificationReport {
                test_entity_ids_digest: digest_identity_set(&["different-test-id".into()]),
                ..report.clone()
            }
            .satisfies(&expected)
        );
        assert!(
            !GraphVerificationReport {
                test_relationship_ids_digest: digest_identity_set(&[
                    "different-test-relationship".into()
                ]),
                ..report.clone()
            }
            .satisfies(&expected)
        );
        assert!(!report.satisfies(&GraphVerificationExpectation {
            code_entities: 3,
            ..expected.clone()
        }));
        assert!(!report.satisfies(&GraphVerificationExpectation {
            generation_id: "generation-other".into(),
            ..expected.clone()
        }));
        assert!(!report.satisfies(&GraphVerificationExpectation {
            context: RepositoryContext {
                repository_id: "other-repository".into(),
                ..expected.context.clone()
            },
            ..expected
        }));
    }
}
