//! Level 3 MemoryRepository test using evidence from a real, indexed CodeMind
//! symbol and the same workspace partition as its Code/History/Test/Change data.

mod support;

use chrono::Utc;
use codemind_engine::{
    graph_storage::MemoryRepository,
    memory::model::{EvidenceReference, MemoryRecord},
};
use serde_json::json;

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn repository_memory_persists_real_snapshot_evidence_and_respects_workspace_partition() {
    let prepared = support::prepare_local_codemind("memory-persistence").await;
    let _analysis = support::run_verified_investigation(&prepared).await;
    let context = prepared.knowledge.context.clone();
    let now = Utc::now().to_rfc3339();
    let record = MemoryRecord {
        memory_id: format!("level3-memory-{}", prepared.run_id),
        owner_id: context.repository_id.clone(),
        kind: "semantic".into(),
        scope: "repository".into(),
        lifecycle: "validated".into(),
        freshness: "fresh".into(),
        payload_json: serde_json::to_string(&json!({
            "summary": format!("CodeMind symbol {} is indexed and test-targeted", prepared.selected_target.name),
            "symbol_id": prepared.selected_target.entity_id.clone(),
            "source_path": prepared.selected_target.path.clone(),
        }))
        .unwrap(),
        context: context.clone(),
        evidence: vec![EvidenceReference {
            path: prepared.selected_target.path.clone(),
            start_line: prepared.selected_target.line,
            end_line: prepared.selected_target.line,
            snapshot_id: context.source_snapshot_id.clone(),
        }],
        idempotency_key: format!("level3-memory-idempotency-{}", prepared.run_id),
        version: 1,
        created_at: now.clone(),
        updated_at: now,
    };

    let persisted = prepared.storage.persist_memory(&record).await.unwrap();
    assert_eq!(persisted.memory_id, record.memory_id);
    assert_eq!(persisted.context, context);
    assert_eq!(persisted.evidence, record.evidence);

    // A new adapter sees the same record, proving it was persisted rather than
    // retained in a process-local repository object.
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
    let matching = retrieved
        .iter()
        .find(|candidate| candidate.memory_id == record.memory_id)
        .expect("persisted memory record must be readable after adapter reconnection");
    assert_eq!(matching.evidence[0].path, prepared.selected_target.path);
    assert_eq!(matching.evidence[0].snapshot_id, context.source_snapshot_id);

    // The same repository/snapshot identity in another workspace must not see
    // this memory because workspace_id selects the physical partition.
    let mut other_workspace = context.clone();
    other_workspace.workspace_id = format!("{}-other", context.workspace_id);
    let isolated = fresh_storage
        .retrieve_memory(
            &other_workspace,
            "repository",
            "semantic",
            &other_workspace.repository_id,
            100,
        )
        .await
        .unwrap();
    assert!(
        !isolated
            .iter()
            .any(|candidate| candidate.memory_id == record.memory_id),
        "repository memory crossed a workspace storage boundary"
    );

    let manifest_path = support::persist_run_manifest(
        &prepared.run_id,
        &json!({
            "run_id": prepared.run_id.clone(),
            "repository_id": context.repository_id.clone(),
            "workspace_id": context.workspace_id.clone(),
            "physical_falkordb_graph": support::physical_graph_name(&context.workspace_id),
            "source_snapshot_id": context.source_snapshot_id.clone(),
            "memory_id": record.memory_id.clone(),
            "evidence_path": prepared.selected_target.path.clone(),
            "evidence_line": prepared.selected_target.line,
            "memory_retrieved_after_reconnect": true,
            "other_workspace_isolated": true,
        }),
    );
    println!("Level 3 memory manifest: {}", manifest_path.display());
}
