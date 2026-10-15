use codemind_engine::{
    engines::tie::{TestRelationshipKind, TestStatus, TieEngine},
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
}
