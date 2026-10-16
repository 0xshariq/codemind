use super::{
    model::{HandoffRecord, MemoryRecord, ReviewContextRecord},
    repository::MemoryRepository,
    validation,
};
use crate::graph_storage::StorageError;

pub async fn persist_memory<R: MemoryRepository>(
    repository: &R,
    record: MemoryRecord,
) -> Result<MemoryRecord, StorageError> {
    validation::validate_memory(&record).map_err(StorageError::Query)?;
    repository.persist_memory(&record).await
}

pub async fn retrieve_memory<R: MemoryRepository>(
    repository: &R,
    context: &crate::source::RepositoryContext,
    scope: &str,
    kind: &str,
    owner_id: &str,
    limit: usize,
) -> Result<Vec<MemoryRecord>, StorageError> {
    validation::validate_context(context).map_err(StorageError::Query)?;
    repository
        .retrieve_memory(context, scope, kind, owner_id, limit)
        .await
}

pub async fn persist_handoff<R: MemoryRepository>(
    repository: &R,
    record: HandoffRecord,
) -> Result<HandoffRecord, StorageError> {
    validation::validate_handoff(&record).map_err(StorageError::Query)?;
    repository.persist_handoff(&record).await
}

pub async fn persist_review_context<R: MemoryRepository>(
    repository: &R,
    record: ReviewContextRecord,
) -> Result<ReviewContextRecord, StorageError> {
    validation::validate_context(&record.context).map_err(StorageError::Query)?;
    let payload_size = serde_json::to_vec(&record)
        .map_err(|error| StorageError::Query(error.to_string()))?
        .len();
    if record.context_id.trim().is_empty()
        || record.session.session_id.trim().is_empty()
        || record.episode.episode_id.trim().is_empty()
        || record.findings.len() > 64
        || record.turns.len() > 100
        || payload_size > crate::memory::model::MAX_PAYLOAD_BYTES
    {
        return Err(StorageError::Query("review context is invalid".to_owned()));
    }
    repository.persist_review_context(&record).await
}
