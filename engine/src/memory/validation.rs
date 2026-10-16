use super::model::{HandoffRecord, MAX_PAYLOAD_BYTES, MemoryRecord};
use crate::source::RepositoryContext;

pub fn validate_memory(record: &MemoryRecord) -> Result<(), String> {
    if record.memory_id.trim().is_empty()
        || !matches!(record.kind.as_str(), "semantic" | "procedure")
        || record.scope.trim().is_empty()
        || !matches!(
            record.lifecycle.as_str(),
            "candidate" | "validated" | "rejected" | "superseded"
        )
        || !matches!(record.freshness.as_str(), "fresh" | "stale" | "unknown")
        || record.payload_json.len() > MAX_PAYLOAD_BYTES
        || (record.scope == "user" && record.owner_id.trim().is_empty())
    {
        return Err("memory record is invalid or exceeds the bounded schema".to_owned());
    }
    validate_context(&record.context)?;
    if record.lifecycle == "validated" && record.evidence.is_empty() {
        return Err("validated memory requires evidence references".to_owned());
    }
    Ok(())
}

pub fn validate_handoff(record: &HandoffRecord) -> Result<(), String> {
    if record.handoff_id.trim().is_empty()
        || record.schema_version.trim().is_empty()
        || record.operation_id.trim().is_empty()
        || record.review_episode_id.trim().is_empty()
        || record.source_stage.trim().is_empty()
        || record.target_stage.trim().is_empty()
        || record.task.len() > 4_000
        || record.findings.len() > 32
        || !matches!(
            record.status.as_str(),
            "pending" | "claimed" | "consumed" | "expired"
        )
    {
        return Err("handoff record is invalid".to_owned());
    }
    validate_context(&record.context)
}

pub fn validate_context(context: &RepositoryContext) -> Result<(), String> {
    if context.repository_id.trim().is_empty()
        || context.workspace_id.trim().is_empty()
        || context.source_snapshot_id.trim().is_empty()
        || context.graph_version.trim().is_empty()
    {
        Err("repository context is required".to_owned())
    } else {
        Ok(())
    }
}
