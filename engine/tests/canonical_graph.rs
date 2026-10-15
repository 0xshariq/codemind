use codemind_engine::{
    engines::coie::{CoieEngine, EntityKind, RelationshipKind},
    source::SourceProvider,
};
use std::fs;

#[test]
fn extraction_preserves_source_context_and_relationships() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("lib.rs"),
        "fn called() {}\nfn caller() { called(); }\n",
    )
    .unwrap();
    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-a")
        .unwrap();
    let graph = CoieEngine.extract(&snapshot).unwrap();

    assert_eq!(graph.context.workspace_id, "workspace-a");
    assert!(
        graph
            .entities
            .iter()
            .any(|entity| entity.kind == EntityKind::File)
    );
    assert!(graph.entities.iter().any(|entity| entity.name == "called"));
    assert!(
        graph
            .relationships
            .iter()
            .any(|edge| edge.kind == RelationshipKind::Defines)
    );
    assert!(
        graph
            .relationships
            .iter()
            .any(|edge| edge.kind == RelationshipKind::Calls)
    );
}
