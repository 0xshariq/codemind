use super::{
    model::{HandoffRecord, MemoryRecord, ReviewContextRecord},
    repository::MemoryRepository,
    scope::AccessContext,
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

pub async fn persist_memory_authorized<R: MemoryRepository>(
    repository: &R,
    access: &AccessContext,
    record: MemoryRecord,
) -> Result<MemoryRecord, StorageError> {
    validation::validate_memory(&record).map_err(StorageError::Query)?;
    if !access.allows_record(&record.scope, &record.owner_id, &record.context) {
        return Err(StorageError::Query(
            "memory scope authorization denied".to_owned(),
        ));
    }
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
    if !matches!(scope, "user" | "team" | "repository") || owner_id.trim().is_empty() {
        return Err(StorageError::Query(
            "memory scope and owner are required".to_owned(),
        ));
    }
    repository
        .retrieve_memory(context, scope, kind, owner_id, limit.min(100))
        .await
}

pub async fn retrieve_memory_authorized<R: MemoryRepository>(
    repository: &R,
    access: &AccessContext,
    context: &crate::source::RepositoryContext,
    scope: &str,
    kind: &str,
    owner_id: &str,
    limit: usize,
) -> Result<Vec<MemoryRecord>, StorageError> {
    validation::validate_context(context).map_err(StorageError::Query)?;
    if access.requested_scope != scope || !access.allows_record(scope, owner_id, context) {
        return Err(StorageError::Query(
            "memory scope authorization denied".to_owned(),
        ));
    }
    let records = repository
        .retrieve_memory(context, scope, kind, owner_id, limit.min(100))
        .await?;
    Ok(records
        .into_iter()
        .filter(|record| access.allows_record(&record.scope, &record.owner_id, &record.context))
        .collect())
}

pub async fn persist_handoff<R: MemoryRepository>(
    repository: &R,
    record: HandoffRecord,
) -> Result<HandoffRecord, StorageError> {
    validation::validate_handoff(&record).map_err(StorageError::Query)?;
    repository.persist_handoff(&record).await
}

pub async fn persist_handoff_authorized<R: MemoryRepository>(
    repository: &R,
    access: &AccessContext,
    record: HandoffRecord,
) -> Result<HandoffRecord, StorageError> {
    validation::validate_handoff(&record).map_err(StorageError::Query)?;
    if access.workspace_id != record.context.workspace_id {
        return Err(StorageError::Query(
            "handoff scope authorization denied".to_owned(),
        ));
    }
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
        || payload_size > crate::memory::model::MAX_REVIEW_CONTEXT_BYTES
    {
        return Err(StorageError::Query("review context is invalid".to_owned()));
    }
    repository.persist_review_context(&record).await
}

pub async fn persist_review_context_authorized<R: MemoryRepository>(
    repository: &R,
    access: &AccessContext,
    record: ReviewContextRecord,
) -> Result<ReviewContextRecord, StorageError> {
    validation::validate_context(&record.context).map_err(StorageError::Query)?;
    if !matches!(record.scope.as_str(), "user" | "team" | "repository")
        || !access.allows_record(&record.scope, &record.owner_id, &record.context)
    {
        return Err(StorageError::Query(
            "review scope authorization denied".to_owned(),
        ));
    }
    persist_review_context(repository, record).await
}
