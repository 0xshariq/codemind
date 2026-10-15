use codemind_engine::{
    engines::{
        chie::{ChieEngine, ImpactClassification, RiskLevel, graph_builder},
        coie::model::{CodeGraph, EntityKind, GraphEntity, GraphRelationship, RelationshipKind},
        tie::model::{TestEntity, TestRelationship, TestRelationshipKind, TestStatus},
    },
    source::RepositoryContext,
};

fn context() -> RepositoryContext {
    RepositoryContext {
        repository_id: "repo".to_owned(),
        workspace_id: "workspace".to_owned(),
        source_snapshot_id: "snapshot".to_owned(),
        graph_version: "day-2".to_owned(),
    }
}

#[test]
fn multi_hop_relationships_produce_direct_indirect_impact_and_tests() {
    let context = context();
    let entities = ["a", "b", "c"]
        .into_iter()
        .map(|id| GraphEntity {
            id: id.to_owned(),
            kind: EntityKind::Symbol,
            name: id.to_owned(),
            path: format!("src/{id}.rs"),
            line: 1,
            context: context.clone(),
        })
        .collect();
    let relationship = |from: &str, to: &str, id: &str| GraphRelationship {
        id: id.to_owned(),
        kind: RelationshipKind::Calls,
        from: from.to_owned(),
        to: to.to_owned(),
        context: context.clone(),
    };
    let graph = CodeGraph {
        context: context.clone(),
        entities,
        relationships: vec![relationship("a", "b", "a-b"), relationship("b", "c", "b-c")],
        diagnostics: Vec::new(),
    };
    let tests = vec![TestEntity {
        id: "test-c".to_owned(),
        name: "test_c".to_owned(),
        path: "tests/c.rs".to_owned(),
        line: 1,
        framework: "rust".to_owned(),
        status: TestStatus::Supported,
        context: context.clone(),
    }];
    let test_relationships = vec![TestRelationship {
        kind: TestRelationshipKind::Targets,
        test_id: "test-c".to_owned(),
        target_symbol_name: Some("c".to_owned()),
        evidence_path: "tests/c.rs".to_owned(),
        context,
    }];

    let analysis = ChieEngine::default()
        .analyze(&graph, &tests, &test_relationships, "a")
        .unwrap();
    assert_eq!(analysis.impact_paths.len(), 2);
    assert_eq!(
        analysis.impact_paths[0].classification,
        ImpactClassification::Direct
    );
    assert_eq!(
        analysis.impact_paths[1].classification,
        ImpactClassification::Indirect
    );
    assert_eq!(analysis.test_candidates.len(), 1);
    assert_eq!(analysis.risk, RiskLevel::Medium);
    assert!(graph_builder::validate(&analysis));
}

#[test]
fn missing_symbol_produces_empty_analysis() {
    let context = context();
    let graph = CodeGraph {
        context,
        entities: Vec::new(),
        relationships: Vec::new(),
        diagnostics: Vec::new(),
    };
    let analysis = ChieEngine::default()
        .analyze(&graph, &[], &[], "missing")
        .unwrap();
    assert!(analysis.impact_paths.is_empty());
    assert_eq!(analysis.risk, RiskLevel::Unknown);
}
