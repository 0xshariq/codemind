use crate::contract::events::EngineEvent;
use crate::contract::operation::OperationRecord;
use crate::engines::chie::{AnalysisRun, ChangeAnalysis, ChangeRecord, ChangeSet};
use crate::engines::coie::model::{CodeGraph, GraphEntity, GraphRelationship};
use crate::engines::tie::model::{TestEntity, TestRelationship};
use crate::source::RepositoryContext;
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedRelationship {
    pub kind: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("graph storage unavailable: {0}")]
    Unavailable(String),
    #[error("graph query failed: {0}")]
    Query(String),
    #[error("invalid graph context")]
    InvalidContext,
}

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
