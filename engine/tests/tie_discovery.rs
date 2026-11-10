use codemind_engine::{
    engines::tie::{TestNodeKind, TestRelationshipKind, TestStatus, TieEngine, graph_builder},
    source::SourceProvider,
};
use std::fs;

#[test]
fn discovers_static_tests_without_claiming_execution() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("lib.rs"), "#[test]\nfn test_called() {}\n").unwrap();
    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-a")
        .unwrap();
    let (tests, relationships) = TieEngine.discover(&snapshot).unwrap();

    assert_eq!(tests.len(), 1);
    assert_eq!(tests[0].status, TestStatus::Supported);
    assert_eq!(relationships[0].kind, TestRelationshipKind::DeclaredBy);
    assert!(
        relationships
            .iter()
            .all(|relationship| { relationship.kind != TestRelationshipKind::Targets })
    );
    assert_generated_tie_ids_are_sha256(&tests, &relationships);
}

#[test]
fn parameterized_javascript_and_typescript_calls_produce_unique_declarations() {
    let root = tempfile::tempdir().unwrap();
    let source = r#"
describe.each(["fast", "slow"])("mode %s", (mode) => {
    test.each([1, 2])("handles %s", (value) => {
        expect(value).toBeDefined();
    });
});

const cases = [["one", 1], ["two", 2]];
test.each(cases).forEach(([name, value]) => {
    test(name, () => expect(value).toBeDefined());
});

test.each(cases).bind(test)("bound %s", (value) => {
    expect(value).toBeDefined();
});
"#;
    fs::write(root.path().join("parameterized.test.js"), source).unwrap();
    fs::write(root.path().join("parameterized.test.ts"), source).unwrap();

    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-parameterized")
        .unwrap();
    let (tests, relationships) = TieEngine.discover(&snapshot).unwrap();

    let suites = tests
        .iter()
        .filter(|test| test.kind == TestNodeKind::TestSuite)
        .count();
    let declarations = tests
        .iter()
        .filter(|test| test.kind == TestNodeKind::Test)
        .count();

    // Each language contributes one describe.each suite, one test.each test,
    // one regular test inside the forEach callback, and one bound test.each
    // declaration. Factory and bind wrapper calls must not become extra nodes.
    assert_eq!(suites, 2);
    assert_eq!(declarations, 6);
    assert_eq!(tests.len(), 8);
    assert_generated_tie_ids_are_sha256(&tests, &relationships);
    graph_builder::validate(&tests, &relationships)
        .expect("TIE declarations from chained APIs must have unique IDs and valid endpoints");
}

#[test]
fn multiple_declarations_in_one_file_have_unique_random_sha256_ids() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("multiple.test.js"),
        "test('first', () => {});\ntest('second', () => {});\n",
    )
    .unwrap();

    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-multiple-tests")
        .unwrap();
    let (tests, relationships) = TieEngine.discover(&snapshot).unwrap();
    let declarations: Vec<_> = tests
        .iter()
        .filter(|test| test.kind == TestNodeKind::Test)
        .collect();

    assert_eq!(declarations.len(), 2);
    assert_generated_tie_ids_are_sha256(&tests, &relationships);
    graph_builder::validate(&tests, &relationships)
        .expect("multiple declarations must produce a valid TIE graph");
}

#[test]
fn nested_same_name_calls_on_one_line_receive_unique_random_sha256_relationship_ids() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("fluent.test.js"),
        "test('fluent calls', () => { client.refresh().refresh(); });\n",
    )
    .unwrap();

    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-fluent-calls")
        .unwrap();
    let (tests, relationships) = TieEngine.discover(&snapshot).unwrap();
    let target_edges = relationships
        .iter()
        .filter(|relationship| relationship.kind == TestRelationshipKind::Targets)
        .collect::<Vec<_>>();

    assert_eq!(target_edges.len(), 2, "both nested calls must be retained");
    assert!(
        relationships
            .iter()
            .all(|relationship| is_sha256_id(&relationship.id, "tie-edge:"))
    );
    assert!(tests.iter().all(|test| {
        let prefix = match test.kind {
            TestNodeKind::Test => "tie-test:",
            TestNodeKind::TestSuite => "tie-suite:",
            TestNodeKind::Fixture => "tie-fixture:",
            TestNodeKind::TestCase => "tie-case:",
        };
        is_sha256_id(&test.id, prefix)
    }));
    graph_builder::validate(&tests, &relationships)
        .expect("nested calls must not produce duplicate TIE relationship IDs");
}

fn assert_generated_tie_ids_are_sha256(
    tests: &[codemind_engine::engines::tie::TestEntity],
    relationships: &[codemind_engine::engines::tie::TestRelationship],
) {
    let mut test_ids = std::collections::HashSet::new();
    for test in tests {
        assert!(
            test_ids.insert(test.id.as_str()),
            "duplicate TIE test ID: {}",
            test.id
        );
        let prefix = match test.kind {
            TestNodeKind::Test => "tie-test:",
            TestNodeKind::TestSuite => "tie-suite:",
            TestNodeKind::Fixture => "tie-fixture:",
            TestNodeKind::TestCase => "tie-case:",
        };
        assert!(
            is_sha256_id(&test.id, prefix),
            "invalid SHA-256 TIE test ID: {}",
            test.id
        );
    }

    let mut relationship_ids = std::collections::HashSet::new();
    for relationship in relationships {
        assert!(
            relationship_ids.insert(relationship.id.as_str()),
            "duplicate TIE relationship ID: {}",
            relationship.id
        );
        assert!(
            is_sha256_id(&relationship.id, "tie-edge:"),
            "invalid SHA-256 TIE relationship ID: {}",
            relationship.id
        );
    }
}

fn is_sha256_id(id: &str, prefix: &str) -> bool {
    let Some(digest) = id.strip_prefix(prefix) else {
        return false;
    };
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[test]
fn rust_parameterized_cases_receive_random_sha256_node_and_edge_ids() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("cases.rs"),
        "#[rstest]\n#[case(1)]\n#[case(2)]\nfn handles_case(#[case] value: i32) { assert!(value > 0); }\n",
    )
    .unwrap();

    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-rust-cases")
        .unwrap();
    let (tests, relationships) = TieEngine.discover(&snapshot).unwrap();
    let cases = tests
        .iter()
        .filter(|test| test.kind == TestNodeKind::TestCase)
        .collect::<Vec<_>>();

    assert_eq!(cases.len(), 2);
    assert_generated_tie_ids_are_sha256(&tests, &relationships);
    graph_builder::validate(&tests, &relationships)
        .expect("parameterized Rust cases must produce unique hashed TIE identities");
}

#[test]
fn python_fixtures_and_their_relationships_receive_hashed_ids() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("test_fixtures.py"),
        "import pytest\n\n@pytest.fixture\ndef database():\n    return object()\n\ndef test_reads_database(database):\n    assert database is not None\n",
    )
    .unwrap();

    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-python-fixtures")
        .unwrap();
    let (tests, relationships) = TieEngine.discover(&snapshot).unwrap();

    assert!(tests.iter().any(|test| test.kind == TestNodeKind::Fixture));
    assert!(
        relationships
            .iter()
            .any(|relationship| relationship.kind == TestRelationshipKind::UsesFixture)
    );
    assert_generated_tie_ids_are_sha256(&tests, &relationships);
    graph_builder::validate(&tests, &relationships)
        .expect("Python fixture graph must use unique SHA-256 TIE identities");
}
