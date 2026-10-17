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
    let called = graph
        .entities
        .iter()
        .find(|entity| entity.name == "called")
        .unwrap();
    let caller = graph
        .entities
        .iter()
        .find(|entity| entity.name == "caller")
        .unwrap();
    assert!(graph.relationships.iter().any(|edge| {
        edge.kind == RelationshipKind::Calls && edge.from == caller.id && edge.to == called.id
    }));
}

#[test]
fn repeated_resolution_preserves_canonical_snapshot_and_entity_ids() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("lib.rs"), "fn called() {}\n").unwrap();
    let provider = SourceProvider::default();
    let first = provider.resolve(root.path(), "workspace-a").unwrap();
    let second = provider.resolve(root.path(), "workspace-a").unwrap();
    let first_graph = CoieEngine.extract(&first).unwrap();
    let second_graph = CoieEngine.extract(&second).unwrap();

    assert_eq!(
        first.context.source_snapshot_id,
        second.context.source_snapshot_id
    );
    assert_eq!(
        first_graph
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>(),
        second_graph
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>()
    );
}
