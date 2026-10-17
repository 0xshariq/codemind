use super::model::{HandoffRecord, MAX_PAYLOAD_BYTES, MemoryRecord};
use crate::source::RepositoryContext;
use std::path::Path;

pub fn validate_memory(record: &MemoryRecord) -> Result<(), String> {
    if record.memory_id.trim().is_empty()
        || record.idempotency_key.trim().is_empty()
        || !matches!(record.kind.as_str(), "semantic" | "procedure")
        || !matches!(record.scope.as_str(), "user" | "team" | "repository")
        || !matches!(
            record.lifecycle.as_str(),
            "candidate" | "validated" | "rejected" | "superseded"
        )
        || !matches!(record.freshness.as_str(), "fresh" | "stale" | "unknown")
        || record.payload_json.len() > MAX_PAYLOAD_BYTES
        || record.owner_id.len() > 512
        || (matches!(record.scope.as_str(), "user" | "team") && record.owner_id.trim().is_empty())
        || record.evidence.len() > 100
    {
        return Err("memory record is invalid or exceeds the bounded schema".to_owned());
    }

    validate_context(&record.context)?;

    for evidence in &record.evidence {
        if evidence.path.trim().is_empty()
            || evidence.path.contains('\\')
            || !crate::source::limits::is_safe_relative(Path::new(&evidence.path))
            || evidence.start_line == 0
            || evidence.end_line < evidence.start_line
            || evidence.snapshot_id != record.context.source_snapshot_id
        {
            return Err("memory evidence is invalid or belongs to another snapshot".to_owned());
        }
    }

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
        || record.idempotency_key.trim().is_empty()
        || record.task.len() > 4_000
        || record.findings.len() > 32
        || record.evidence_references.len() > 100
        || record.open_questions.len() > 64
        || !matches!(
            record.status.as_str(),
            "pending" | "claimed" | "consumed" | "expired"
        )
    {
        return Err("handoff record is invalid".to_owned());
    }

    validate_context(&record.context)?;
    for evidence in &record.evidence_references {
        if evidence.path.trim().is_empty()
            || evidence.path.contains('\\')
            || !crate::source::limits::is_safe_relative(Path::new(&evidence.path))
            || evidence.start_line == 0
            || evidence.end_line < evidence.start_line
            || evidence.snapshot_id != record.context.source_snapshot_id
        {
            return Err("handoff evidence is invalid or belongs to another snapshot".to_owned());
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::validate_memory;
    use crate::memory::model::{EvidenceReference, MemoryRecord};
    use crate::source::RepositoryContext;

    fn record(lifecycle: &str, evidence: Vec<EvidenceReference>) -> MemoryRecord {
        MemoryRecord {
            memory_id: "memory".into(),
            owner_id: "owner".into(),
            kind: "semantic".into(),
            scope: "user".into(),
            lifecycle: lifecycle.into(),
            freshness: "fresh".into(),
            payload_json: "{}".into(),
            context: RepositoryContext {
                repository_id: "repo".into(),
                workspace_id: "workspace".into(),
                source_snapshot_id: "snapshot".into(),
                graph_version: "graph".into(),
            },
            evidence,
            idempotency_key: "idem".into(),
            version: 1,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn validated_memory_requires_same_snapshot_evidence() {
        let evidence = EvidenceReference {
            path: "src/lib.rs".into(),
            start_line: 1,
            end_line: 2,
            snapshot_id: "snapshot".into(),
        };
        assert!(validate_memory(&record("validated", vec![evidence.clone()])).is_ok());
        let wrong = EvidenceReference {
            snapshot_id: "other".into(),
            ..evidence
        };
        assert!(validate_memory(&record("validated", vec![wrong])).is_err());
        assert!(validate_memory(&record("validated", Vec::new())).is_err());
    }
}
