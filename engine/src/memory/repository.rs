use super::model::{HandoffRecord, MemoryRecord, ReviewContextRecord};
use crate::graph_storage::StorageError;
use async_trait::async_trait;

#[async_trait]
pub trait MemoryRepository: Send + Sync {
    async fn persist_memory(&self, record: &MemoryRecord) -> Result<MemoryRecord, StorageError>;
    async fn retrieve_memory(
        &self,
        context: &crate::source::RepositoryContext,
        scope: &str,
        kind: &str,
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
