use codemind_engine::{
    engines::coie::model::{CodeGraph, EntityKind, GraphEntity, ResolutionStatus},
    graph_storage::{FalkorDbStorage, GraphStorage},
    memory::{
        model::{EvidenceReference, MemoryRecord},
        repository::MemoryRepository,
    },
    source::RepositoryContext,
};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires FalkorDB; run with --ignored"]
async fn memory_survives_a_fresh_repository_lookup_with_scope_and_evidence() {
    let graph_name = format!("codemind_level3_memory_{}", Uuid::new_v4().simple());

    let storage = {
        let url = std::env::var("FALKORDB_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".into());

        FalkorDbStorage::connect(&url, &graph_name).await.unwrap()
    };

    let context = RepositoryContext {
        repository_id: "repo-memory".into(),
        workspace_id: "workspace-memory".into(),
        source_snapshot_id: "snapshot-memory".into(),
        graph_version: "graph-memory".into(),
    };

    let record = MemoryRecord {
        memory_id: format!("memory:{}", graph_name),
        owner_id: "repo-memory".into(),
        kind: "semantic".into(),
        scope: "repository".into(),
        lifecycle: "validated".into(),
        freshness: "fresh".into(),
        payload_json: r#"{"summary":"Code graph is snapshot pinned"}"#.into(),
        context: context.clone(),
        evidence: vec![EvidenceReference {
            path: "src/lib.rs".into(),
            start_line: 1,
            end_line: 2,
            snapshot_id: context.source_snapshot_id.clone(),
        }],
        idempotency_key: format!("idem:{}", graph_name),
        version: 1,
        created_at: "2026-10-17T00:00:00Z".into(),
        updated_at: "2026-10-17T00:00:00Z".into(),
    };

    // FalkorDB requires the graph key to be materialized before a read-only
    // query can be executed. Memory persistence performs a read-before-write
    // idempotency check, so bootstrap this disposable graph with a minimal
    // canonical write first. This keeps the test focused on memory behavior
    // rather than FalkorDB's empty-graph semantics.
    let bootstrap_graph = CodeGraph {
        context: context.clone(),
        entities: vec![GraphEntity {
            id: format!("bootstrap:{}", graph_name),
            kind: EntityKind::File,
            name: "src/lib.rs".into(),
            path: "src/lib.rs".into(),
            line: 1,
            context: context.clone(),
            language: Some(codemind_engine::source::Language::Rust),
            native_kind: Some("source_file".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        }],
        relationships: Vec::new(),
        diagnostics: Vec::new(),
    };

    storage.write_code_graph(&bootstrap_graph).await.unwrap();

    let graph_storage: &dyn GraphStorage = &storage;

    let persisted = MemoryRepository::persist_memory(&storage, &record)
        .await
        .unwrap();

    assert_eq!(persisted.memory_id, record.memory_id);

    let retrieved = MemoryRepository::retrieve_memory(
        graph_storage,
        &context,
        "repository",
        "semantic",
        "repo-memory",
        10,
    )
    .await
    .unwrap();

    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].memory_id, record.memory_id);
    assert_eq!(
        retrieved[0].evidence[0].snapshot_id,
        context.source_snapshot_id
    );
}
