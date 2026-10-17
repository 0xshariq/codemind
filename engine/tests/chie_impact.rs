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
            language: Some(codemind_engine::source::Language::Rust),
            native_kind: Some("function_item".to_owned()),
            range: None,
            resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
            evidence: Some(codemind_engine::engines::coie::model::SourceEvidence {
                path: format!("src/{id}.rs"),
                start_line: 1,
                end_line: 1,
                snapshot_id: context.source_snapshot_id.clone(),
                provenance: "test".to_owned(),
            }),
        })
        .collect();

    let relationship = |from: &str, to: &str, id: &str| GraphRelationship {
        id: id.to_owned(),
        kind: RelationshipKind::Calls,
        from: from.to_owned(),
        to: to.to_owned(),
        context: context.clone(),
        resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
        evidence: Some(codemind_engine::engines::coie::model::SourceEvidence {
            path: "src/lib.rs".to_owned(),
            start_line: 1,
            end_line: 1,
            snapshot_id: context.source_snapshot_id.clone(),
            provenance: "test".to_owned(),
        }),
    };

    let graph = CodeGraph {
        context: context.clone(),
        entities,
        relationships: vec![relationship("b", "a", "b-a"), relationship("c", "b", "c-b")],
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
        id: "target-c".to_owned(),
        kind: TestRelationshipKind::Targets,
        test_id: "test-c".to_owned(),
        target_symbol_name: Some("c".to_owned()),
        entity_id: Some("c".to_owned()),
        evidence_path: "tests/c.rs".to_owned(),
        evidence: Some(codemind_engine::engines::coie::model::SourceEvidence {
            path: "tests/c.rs".to_owned(),
            start_line: 1,
            end_line: 1,
            snapshot_id: context.source_snapshot_id.clone(),
            provenance: "test".to_owned(),
        }),
        resolution: codemind_engine::engines::tie::model::TestResolution::Resolved,
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

    assert!(graph_builder::validate(&analysis).is_ok());
}

#[test]
fn missing_symbol_produces_selection_error() {
    let context = context();

    let graph = CodeGraph {
        context,
        entities: Vec::new(),
        relationships: Vec::new(),
        diagnostics: Vec::new(),
    };

    let result = ChieEngine::default().analyze(&graph, &[], &[], "missing");

    assert!(
        result.is_err(),
        "expected ChIE to reject a selected symbol that does not exist in the code graph"
    );

    assert_eq!(
        result.unwrap_err(),
        "selected symbol or file does not exist in the code graph"
    );
}
