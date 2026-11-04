use super::model::{HandoffRecord, MemoryRecord, ReviewContextRecord};
use crate::graph_storage::StorageError;
use async_trait::async_trait;

/// Workspace-partitioned persistence for Agent Memory and review handoffs.
///
/// The workspace in each record/context selects its physical storage partition.
/// Scope and owner authorization remain separate checks and must not be inferred
/// from the fact that two records reside in the same workspace graph.
#[async_trait]
pub trait MemoryRepository: Send + Sync {
    async fn persist_memory(&self, record: &MemoryRecord) -> Result<MemoryRecord, StorageError>;
    async fn retrieve_memory(
        &self,
        context: &crate::source::RepositoryContext,
        scope: &str,
        kind: &str,
        owner_id: &str,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, StorageError>;
    async fn persist_handoff(&self, record: &HandoffRecord) -> Result<HandoffRecord, StorageError>;
    async fn retrieve_handoff(
        &self,
        context: &crate::source::RepositoryContext,
        handoff_id: &str,
    ) -> Result<HandoffRecord, StorageError>;
    async fn transition_handoff(
        &self,
        context: &crate::source::RepositoryContext,
        handoff_id: &str,
        status: &str,
    ) -> Result<HandoffRecord, StorageError>;
    async fn persist_review_context(
        &self,
        record: &ReviewContextRecord,
    ) -> Result<ReviewContextRecord, StorageError>;
    async fn retrieve_review_context(
        &self,
        context: &crate::source::RepositoryContext,
        context_id: &str,
    ) -> Result<ReviewContextRecord, StorageError>;
}
