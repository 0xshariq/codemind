use codemind_engine::{
    engines::{
        coie::{
            extraction::{Candidate, normalize::identifier},
            graph_builder, languages,
            model::{CodeGraph, EntityKind, GraphEntity},
            queries::{paths, symbols},
        },
        tie::{
            discovery::{javascript_tests, python_tests, rust_tests, typescript_tests},
            graph_builder as tie_graph_builder,
            mapping::{evidence, target_mapping},
            model::{TestEntity, TestStatus},
        },
    },
    source::{Language, RepositoryContext},
};

fn context() -> RepositoryContext {
    RepositoryContext {
        repository_id: "repository".to_owned(),
        workspace_id: "workspace".to_owned(),
        source_snapshot_id: "snapshot".to_owned(),
        graph_version: "day-1".to_owned(),
    }
}

#[test]
fn coie_boundaries_normalize_validate_and_query_canonical_data() {
    assert_eq!(identifier("symbol_name"), Some("symbol_name".to_owned()));
    assert!(languages::supports(Language::Rust));

    let file = GraphEntity {
        id: "file".to_owned(),
        kind: EntityKind::File,
        name: "lib.rs".to_owned(),
        path: "lib.rs".to_owned(),
        line: 1,
        context: context(),
        language: Some(Language::Rust),
        native_kind: Some("source_file".to_owned()),
        range: None,
        resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
        evidence: None,
    };

    let graph = CodeGraph {
        context: context(),
        entities: vec![file],
        relationships: Vec::new(),
        diagnostics: Vec::new(),
    };

    assert!(graph_builder::validate(&graph));
    assert_eq!(symbols::symbols(&graph.entities).count(), 0);
    assert_eq!(paths::outgoing(&graph.relationships, "file").count(), 0);
}

#[test]
fn language_and_test_boundaries_preserve_supported_semantics() {
    let candidate = Candidate {
        name: "run".to_owned(),
        path: "src/lib.rs".to_owned(),
        language: Language::Rust,
        native_kind: "function_item".to_owned(),
        line: 4,
        range: codemind_engine::source::tree_sitter::SourceRange {
            start_byte: 0,
            end_byte: 3,
            start_line: 4,
            end_line: 4,
            start_column: 1,
            end_column: 4,
        },
        context: context(),
        enclosing: None,
        resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
    };

    assert!(codemind_engine::engines::coie::extraction::validate::supported(&candidate));

    assert!(rust_tests::is_test_declaration("#[test]\nfn works() {}"));

    assert!(javascript_tests::is_test_declaration(
        "test('works', () => {})"
    ));

    assert!(typescript_tests::is_test_declaration(
        "it('works', () => {})"
    ));

    assert!(python_tests::is_test_declaration("def test_works():"));

    assert!(target_mapping::supported(
        codemind_engine::engines::tie::model::TestRelationshipKind::Targets
    ));

    assert_eq!(
        evidence::normalize_path(r"tests\unit.py"),
        Some("tests/unit.py".to_owned())
    );

    let test = TestEntity {
        id: "test".to_owned(),
        name: "test_works".to_owned(),
        path: "tests/unit.py".to_owned(),
        line: 1,
        framework: "pytest-unittest".to_owned(),
        status: TestStatus::Supported,
        context: context(),
    };

    assert!(tie_graph_builder::validate(&[test], &[]).is_ok());
}
