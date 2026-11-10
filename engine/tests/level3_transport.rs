mod support;

use codemind_engine::memory::{
    model::{EvidenceReference, MemoryRecord},
    scope::allows,
    validation::validate_memory,
};

#[test]
fn validated_memory_requires_bounded_evidence_and_context() {
    let record = MemoryRecord {
        memory_id: "memory-1".into(),
        // Repository-scoped records must be unowned or owned by the repository.
        // support::test_repository_context() uses "repo" as repository_id.
        owner_id: "repo".into(),
        kind: "semantic".into(),
        scope: "repository".into(),
        lifecycle: "validated".into(),
        freshness: "fresh".into(),
        payload_json: r#"{"summary":"control plane callers"}"#.into(),
        context: support::test_repository_context("snapshot-a"),
        evidence: vec![EvidenceReference {
            path: "src/control.rs".into(),
            start_line: 10,
            end_line: 18,
            snapshot_id: "snapshot-a".into(),
        }],
        idempotency_key: "memory-1".into(),
        version: 1,
        created_at: "2026-10-17T00:00:00Z".into(),
        updated_at: "2026-10-17T00:00:00Z".into(),
    };
    assert!(validate_memory(&record).is_ok());
    assert!(allows("repository", "repository", &record.context));
    assert!(!allows("team", "repository", &record.context));
}

#[test]
fn validated_memory_without_evidence_is_rejected() {
    let record = MemoryRecord {
        memory_id: "memory-2".into(),
        owner_id: "user-1".into(),
        kind: "procedure".into(),
        scope: "repository".into(),
        lifecycle: "validated".into(),
        freshness: "fresh".into(),
        payload_json: "{}".into(),
        context: support::test_repository_context("snapshot-a"),
        evidence: Vec::new(),
        idempotency_key: "memory-2".into(),
        version: 1,
        created_at: String::new(),
        updated_at: String::new(),
    };
    assert!(validate_memory(&record).is_err());
}
