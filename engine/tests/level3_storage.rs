mod support;

use codemind_engine::{
    engines::coie::model::{
        CodeGraph, EntityKind, GraphEntity, GraphRelationship, RelationshipKind, ResolutionStatus,
    },
    graph_storage::GraphStorage,
    source::RepositoryContext,
};
use uuid::Uuid;

fn context() -> RepositoryContext {
    RepositoryContext {
        repository_id: "level3-storage-repository".into(),
        workspace_id: "level3-storage".into(),
        source_snapshot_id: "level3-storage-snapshot".into(),
        graph_version: "level3-storage-graph".into(),
    }
}

fn symbol(context: &RepositoryContext, id: &str, name: &str) -> GraphEntity {
    GraphEntity {
        id: id.into(),
        kind: EntityKind::Symbol,
        name: name.into(),
        path: format!("src/{name}.rs"),
        line: 1,
        context: context.clone(),
        language: Some(codemind_engine::source::Language::Rust),
        native_kind: Some("function_item".into()),
        range: None,
        resolution: ResolutionStatus::Resolved,
        evidence: Some(codemind_engine::engines::coie::model::SourceEvidence {
            path: format!("src/{name}.rs"),
            start_line: 1,
            end_line: 1,
            snapshot_id: context.source_snapshot_id.clone(),
            provenance: "level3".into(),
        }),
    }
}

#[tokio::test]
#[ignore = "requires FalkorDB; run with --ignored"]
async fn code_graph_reingestion_removes_stale_canonical_relationships() {
    let graph_name = format!("codemind_level3_stale_{}", Uuid::new_v4().simple());
    let storage = support::storage(&graph_name).await;
    let context = context();
    let a = symbol(&context, "a", "a");
    let b = symbol(&context, "b", "b");
    let c = symbol(&context, "c", "c");
    let evidence = Some(codemind_engine::engines::coie::model::SourceEvidence {
        path: "src/a.rs".into(),
        start_line: 1,
        end_line: 1,
        snapshot_id: context.source_snapshot_id.clone(),
        provenance: "level3".into(),
    });

    let first = CodeGraph {
        context: context.clone(),
        entities: vec![a.clone(), b.clone(), c.clone()],
        relationships: vec![
            GraphRelationship {
                id: "a-b".into(),
                kind: RelationshipKind::Calls,
                from: a.id.clone(),
                to: b.id.clone(),
                context: context.clone(),
                resolution: ResolutionStatus::Resolved,
                evidence: evidence.clone(),
            },
            GraphRelationship {
                id: "b-c".into(),
                kind: RelationshipKind::Calls,
                from: b.id.clone(),
                to: c.id.clone(),
                context: context.clone(),
                resolution: ResolutionStatus::Resolved,
                evidence: evidence.clone(),
            },
        ],
        diagnostics: Vec::new(),
    };
    storage.write_code_graph(&first).await.unwrap();
    assert_eq!(
        storage
            .relationships_for(&context, "a", 20)
            .await
            .unwrap()
            .len(),
        1
    );

    let second = CodeGraph {
        context: context.clone(),
        entities: vec![a.clone(), b.clone(), c.clone()],
        relationships: vec![GraphRelationship {
            id: "a-b".into(),
            kind: RelationshipKind::Calls,
            from: a.id.clone(),
            to: b.id.clone(),
            context: context.clone(),
            resolution: ResolutionStatus::Resolved,
            evidence: evidence.clone(),
        }],
        diagnostics: Vec::new(),
    };
    storage.write_code_graph(&second).await.unwrap();
    assert!(
        storage
            .relationships_for(&context, "b", 20)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
#[ignore = "requires FalkorDB; run with --ignored"]
async fn empty_test_graph_is_a_valid_snapshot_and_clears_previous_tests() {
    let graph_name = format!("codemind_level3_empty_tie_{}", Uuid::new_v4().simple());
    let storage = support::storage(&graph_name).await;
    let context = context();
    let code = symbol(&context, "target", "target");
    let graph = CodeGraph {
        context: context.clone(),
        entities: vec![code.clone()],
        relationships: Vec::new(),
        diagnostics: Vec::new(),
    };
    storage.write_code_graph(&graph).await.unwrap();

    let test = codemind_engine::engines::tie::model::TestEntity {
        id: "test-1".into(),
        name: "test_target".into(),
        path: "tests/target.rs".into(),
        line: 1,
        framework: "rust".into(),
        status: codemind_engine::engines::tie::model::TestStatus::Supported,
        context: context.clone(),
    };
    let relationship = codemind_engine::engines::tie::model::TestRelationship {
        id: "target-1".into(),
        kind: codemind_engine::engines::tie::model::TestRelationshipKind::Targets,
        test_id: test.id.clone(),
        target_symbol_name: Some("target".into()),
        entity_id: Some(code.id.clone()),
        evidence_path: "tests/target.rs".into(),
        evidence: None,
        resolution: codemind_engine::engines::tie::model::TestResolution::Resolved,
        context: context.clone(),
    };
    storage
        .write_test_graph(&context, &[test], &[relationship])
        .await
        .unwrap();
    let target_ids = vec![code.id.clone()];
    assert!(
        !storage
            .tests_for_context(&context, &target_ids, 20)
            .await
            .unwrap()
            .tests
            .is_empty()
    );

    storage.write_test_graph(&context, &[], &[]).await.unwrap();
    assert!(
        storage
            .tests_for_context(&context, &target_ids, 20)
            .await
            .unwrap()
            .tests
            .is_empty()
    );
}
