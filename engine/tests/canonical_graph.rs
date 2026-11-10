use codemind_engine::{
    engines::{
        chie::{ChieEngine, ImpactClassification, RiskLevel, graph_builder},
        coie::model::{CodeGraph, EntityKind, GraphEntity, GraphRelationship, RelationshipKind},
        tie::model::{TestEntity, TestRelationship, TestRelationshipKind, TestStatus},
    },
    source::{RepositoryContext, identity::RandomIdGenerator},
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

    let mut id_generator = RandomIdGenerator::default();
    let entity_a = id_generator.next("coie-entity", "fixture-symbol");
    let entity_b = id_generator.next("coie-entity", "fixture-symbol");
    let entity_c = id_generator.next("coie-entity", "fixture-symbol");
    let entities = [
        ("a", entity_a.as_str()),
        ("b", entity_b.as_str()),
        ("c", entity_c.as_str()),
    ]
    .into_iter()
    .map(|(name, id)| GraphEntity {
        id: id.to_owned(),
        kind: EntityKind::Symbol,
        name: name.to_owned(),
        path: format!("src/{name}.rs"),
        line: 1,
        context: context.clone(),
        language: Some(codemind_engine::source::Language::Rust),
        native_kind: Some("function_item".to_owned()),
        range: None,
        resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
        evidence: Some(codemind_engine::engines::coie::model::SourceEvidence {
            path: format!("src/{name}.rs"),
            start_line: 1,
            end_line: 1,
            snapshot_id: context.source_snapshot_id.clone(),
            provenance: "test".to_owned(),
        }),
    })
    .collect();

    let mut relationship = |from: &str, to: &str| GraphRelationship {
        id: id_generator.next("coie-relationship", "calls"),
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
        relationships: vec![
            relationship(&entity_b, &entity_a),
            relationship(&entity_c, &entity_b),
        ],
        diagnostics: Vec::new(),
    };

    let test_id = id_generator.next("tie-test", "fixture-test");
    let target_relationship_id = id_generator.next("tie-edge", "targets");
    let tests = vec![TestEntity {
        id: test_id.clone(),
        name: "test_c".to_owned(),
        path: "tests/c.rs".to_owned(),
        line: 1,
        framework: "rust".to_owned(),
        status: TestStatus::Supported,
        kind: Default::default(),
        context: context.clone(),
    }];

    let test_relationships = vec![TestRelationship {
        id: target_relationship_id,
        kind: TestRelationshipKind::Targets,
        test_id,
        target_symbol_name: Some("c".to_owned()),
        entity_id: Some(entity_c.clone()),
        evidence_path: "tests/c.rs".to_owned(),
        evidence: Some(codemind_engine::engines::coie::model::SourceEvidence {
            path: "tests/c.rs".to_owned(),
            start_line: 1,
            end_line: 1,
            snapshot_id: context.source_snapshot_id.clone(),
            provenance: "test".to_owned(),
        }),
        resolution: codemind_engine::engines::tie::model::TestResolution::Resolved,
        target_kind: Default::default(),
        context,
    }];

    let analysis = ChieEngine::default()
        .analyze(&graph, &tests, &test_relationships, &entity_a)
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
fn risk_counts_unique_impacted_entities_not_alternate_paths() {
    let context = context();

    let mut id_generator = RandomIdGenerator::default();
    let entity_a = id_generator.next("coie-entity", "fixture-symbol");
    let entity_b = id_generator.next("coie-entity", "fixture-symbol");
    let entity_c = id_generator.next("coie-entity", "fixture-symbol");
    let entity_d = id_generator.next("coie-entity", "fixture-symbol");
    let entities = [
        ("a", entity_a.as_str()),
        ("b", entity_b.as_str()),
        ("c", entity_c.as_str()),
        ("d", entity_d.as_str()),
    ]
    .into_iter()
    .map(|(name, id)| GraphEntity {
        id: id.to_owned(),
        kind: EntityKind::Symbol,
        name: name.to_owned(),
        path: format!("src/{name}.rs"),
        line: 1,
        context: context.clone(),
        language: Some(codemind_engine::source::Language::Rust),
        native_kind: Some("function_item".to_owned()),
        range: None,
        resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
        evidence: None,
    })
    .collect();

    let mut edge = |from: &str, to: &str| GraphRelationship {
        id: id_generator.next("coie-relationship", "calls"),
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
        relationships: vec![
            edge(&entity_b, &entity_a),
            edge(&entity_c, &entity_a),
            edge(&entity_d, &entity_b),
            edge(&entity_d, &entity_c),
        ],
        diagnostics: Vec::new(),
    };

    let analysis = ChieEngine::default()
        .analyze(&graph, &[], &[], &entity_a)
        .unwrap();

    assert_eq!(analysis.impact_paths.len(), 4);
    assert_eq!(
        analysis.impact.as_ref().unwrap().impacted_entity_ids.len(),
        3
    );
    assert_eq!(analysis.risk, RiskLevel::Medium);
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
