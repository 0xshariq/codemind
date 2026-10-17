use codemind_engine::{
    engines::coie::CoieEngine,
    engines::coie::model::{
        CodeGraph, EntityKind, GraphEntity, GraphRelationship, RelationshipKind,
    },
    graph_storage::{FalkorDbStorage, GraphStorage},
    source::RepositoryContext,
    source::SourceProvider,
};

fn context(workspace_id: &str) -> RepositoryContext {
    RepositoryContext {
        repository_id: "day-1-integration-repository".to_owned(),
        workspace_id: workspace_id.to_owned(),
        source_snapshot_id: "day-1-integration-snapshot".to_owned(),
        graph_version: "day-1".to_owned(),
    }
}

fn graph(context: &RepositoryContext) -> CodeGraph {
    let file = GraphEntity {
        id: "day-1-file".to_owned(),
        kind: EntityKind::File,
        name: "src/lib.rs".to_owned(),
        path: "src/lib.rs".to_owned(),
        line: 1,
        context: context.clone(),
        language: Some(codemind_engine::source::Language::Rust),
        native_kind: Some("source_file".to_owned()),
        range: None,
        resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
        evidence: None,
    };
    let symbol = GraphEntity {
        id: "day-1-symbol".to_owned(),
        kind: EntityKind::Symbol,
        name: "target".to_owned(),
        path: "src/lib.rs".to_owned(),
        line: 2,
        context: context.clone(),
        language: Some(codemind_engine::source::Language::Rust),
        native_kind: Some("function_item".to_owned()),
        range: None,
        resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
        evidence: Some(codemind_engine::engines::coie::model::SourceEvidence {
            path: "src/lib.rs".to_owned(),
            start_line: 2,
            end_line: 2,
            snapshot_id: context.source_snapshot_id.clone(),
            provenance: "test".to_owned(),
        }),
    };
    CodeGraph {
        context: context.clone(),
        entities: vec![file.clone(), symbol.clone()],
        relationships: vec![GraphRelationship {
            id: "day-1-defines".to_owned(),
            kind: RelationshipKind::Defines,
            from: file.id,
            to: symbol.id,
            context: context.clone(),
            resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
            evidence: Some(codemind_engine::engines::coie::model::SourceEvidence {
                path: "src/lib.rs".to_owned(),
                start_line: 2,
                end_line: 2,
                snapshot_id: context.source_snapshot_id.clone(),
                provenance: "test".to_owned(),
            }),
        }],
        diagnostics: Vec::new(),
    }
}

#[tokio::test]
#[ignore = "requires FalkorDB; run with --ignored"]
async fn writes_reads_isolates_and_repeats_canonical_graph_data() {
    let url = std::env::var("FALKORDB_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_owned());
    let storage = FalkorDbStorage::connect(&url, "codemind_day_1_integration")
        .await
        .expect("FalkorDB must be available for this integration test");
    let workspace_a = context("workspace-a");
    let workspace_b = context("workspace-b");
    let graph_a = graph(&workspace_a);

    storage.write_code_graph(&graph_a).await.unwrap();
    storage.write_code_graph(&graph_a).await.unwrap();

    let relationships = storage
        .relationships_for(&workspace_a, "day-1-file", 10)
        .await
        .unwrap();
    assert_eq!(relationships.len(), 1);
    assert_eq!(relationships[0].kind, RelationshipKind::Defines);
    assert_eq!(relationships[0].from, "day-1-file");
    assert_eq!(relationships[0].to, "day-1-symbol");

    let isolated = storage
        .relationships_for(&workspace_b, "day-1-file", 10)
        .await
        .unwrap();
    assert!(isolated.is_empty());
}

#[tokio::test]
#[ignore = "requires FalkorDB; run with --ignored"]
async fn ingests_developer_source_into_the_real_graph() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("lib.rs"), "fn source_symbol() {}\n").unwrap();
    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-source")
        .unwrap();
    let url = std::env::var("FALKORDB_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_owned());
    let storage = FalkorDbStorage::connect(&url, "codemind_day_1_source")
        .await
        .unwrap();
    let report = CoieEngine.ingest(&snapshot, &storage).await.unwrap();
    assert_eq!(report.context, snapshot.context);
    assert!(report.entity_count > 0);
    assert!(report.relationship_count > 0);

    let file_id = format!(
        "file:{}:{}",
        snapshot.context.source_snapshot_id,
        codemind_engine::source::identity::digest_text("lib.rs")
    );
    let relationships = storage
        .relationships_for(&snapshot.context, &file_id, 10)
        .await
        .unwrap();
    assert!(
        relationships
            .iter()
            .any(|relationship| relationship.kind == RelationshipKind::Defines)
    );
}
