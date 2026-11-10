//! Level 3 memory-storage idempotency test using evidence from a real CodeMind
//! symbol. This exercises MemoryRepository on the same workspace graph used by
//! the production indexing and investigation pipeline.

mod support;

use chrono::Utc;
use codemind_engine::{
    graph_storage::MemoryRepository,
    memory::model::{EvidenceReference, MemoryRecord},
};
use serde_json::json;

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn memory_retries_are_idempotent_and_conflicting_reuse_is_rejected() {
    let prepared = support::prepare_local_codemind("memory-idempotency").await;
    let _investigation = support::run_verified_investigation(&prepared).await;
    let context = prepared.knowledge.context.clone();
    let now = Utc::now().to_rfc3339();
    let record = MemoryRecord {
        memory_id: format!("level3-memory-idem-{}", prepared.run_id),
        owner_id: context.repository_id.clone(),
        kind: "semantic".into(),
        scope: "repository".into(),
        lifecycle: "validated".into(),
        freshness: "fresh".into(),
        payload_json: serde_json::to_string(&json!({
            "summary": format!("Indexed CodeMind symbol {}", prepared.selected_target.name),
            "path": prepared.selected_target.path.clone(),
            "symbol_id": prepared.selected_target.entity_id.clone(),
        }))
        .unwrap(),
        context: context.clone(),
        evidence: vec![EvidenceReference {
            path: prepared.selected_target.path.clone(),
            start_line: prepared.selected_target.line,
            end_line: prepared.selected_target.line,
            snapshot_id: context.source_snapshot_id.clone(),
        }],
        idempotency_key: format!("level3-memory-key-{}", prepared.run_id),
        version: 1,
        created_at: now.clone(),
        updated_at: now,
    };

    let first = prepared.storage.persist_memory(&record).await.unwrap();
    let retry = prepared.storage.persist_memory(&record).await.unwrap();
    assert_eq!(first.memory_id, retry.memory_id);
    assert_eq!(first.payload_json, retry.payload_json);
    assert_eq!(first.evidence, retry.evidence);

    let mut conflicting_payload = record.clone();
    conflicting_payload.payload_json = r#"{"summary":"different payload"}"#.into();
    let conflict = prepared.storage.persist_memory(&conflicting_payload).await;
    assert!(
        conflict.is_err(),
        "reusing the same memory idempotency key with a different payload must fail"
    );

    let mut conflicting_key = record.clone();
    conflicting_key.idempotency_key.push_str("-different");
    let conflict = prepared.storage.persist_memory(&conflicting_key).await;
    assert!(
        conflict.is_err(),
        "reusing an existing memory identity with another idempotency key must fail"
    );

    let fresh_storage = support::connect_falkordb().await;
    let retrieved = fresh_storage
        .retrieve_memory(
            &context,
            "repository",
            "semantic",
            &context.repository_id,
            100,
        )
        .await
        .unwrap();
    assert_eq!(
        retrieved
            .iter()
            .filter(|item| item.memory_id == record.memory_id)
            .count(),
        1,
        "idempotent retry must not create duplicate memory identities"
    );

    let manifest_path = support::persist_run_manifest(
        &prepared.run_id,
        &serde_json::json!({
            "run_id": prepared.run_id.clone(),
            "workspace_id": context.workspace_id.clone(),
            "physical_falkordb_graph": support::physical_graph_name(&context.workspace_id),
            "memory_id": record.memory_id.clone(),
            "retry_idempotent": true,
            "conflicting_payload_rejected": true,
            "conflicting_key_rejected": true,
            "persisted_identity_count": retrieved.iter().filter(|item| item.memory_id == record.memory_id).count(),
        }),
    );
    println!("Memory idempotency manifest: {}", manifest_path.display());
}
