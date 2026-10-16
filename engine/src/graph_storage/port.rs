use crate::engines::chie::model::ChangeAnalysis;
use crate::engines::coie::model::{CodeGraph, GraphRelationship};
use crate::engines::tie::model::{TestEntity, TestRelationship};
use crate::source::RepositoryContext;
use async_trait::async_trait;
use thiserror::Error;

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
    async fn write_code_graph(&self, graph: &CodeGraph) -> Result<(), StorageError>;
    async fn write_test_graph(
        &self,
        tests: &[TestEntity],
        relationships: &[TestRelationship],
    ) -> Result<(), StorageError>;
    async fn relationships_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationship>, StorageError>;
    async fn analyze_change(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        max_depth: usize,
    ) -> Result<ChangeAnalysis, StorageError>;
}
