use codemind_engine::{
    contract::context::RepositoryContext,
    engines::coie::{
        CoieEngine, EntityKind, GraphRelationship, RelationshipKind, ResolutionStatus,
        graph_builder, queries::{paths, relationships},
    },
    repository::RepositoryIdentity,
    source::SourceProvider,
};
use std::{collections::HashSet, fs};

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
    graph_builder::validate_detailed(&graph)
        .expect("CoIE must resolve every generated relationship endpoint");
}

#[test]
fn repeated_resolution_preserves_canonical_snapshot_and_mints_fresh_graph_ids() {
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
    graph_builder::validate_detailed(&first_graph).unwrap();
    graph_builder::validate_detailed(&second_graph).unwrap();
    let first_entity_ids: HashSet<_> = first_graph
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect();
    assert!(
        second_graph
            .entities
            .iter()
            .all(|entity| !first_entity_ids.contains(entity.id.as_str())),
        "repeated extraction must mint fresh graph entity IDs"
    );
    assert!(
        first_graph
            .entities
            .iter()
            .any(|entity| entity.kind == EntityKind::Symbol)
    );
    assert!(
        second_graph
            .entities
            .iter()
            .any(|entity| entity.kind == EntityKind::Symbol)
    );
}

#[test]
fn stable_remote_identity_keeps_repository_and_snapshot_across_checkout_paths() {
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    fs::create_dir_all(first_root.path().join("src")).unwrap();
    fs::create_dir_all(second_root.path().join("src")).unwrap();
    let source = "pub fn called() {}\npub fn caller() { called(); }\n";
    fs::write(first_root.path().join("src/lib.rs"), source).unwrap();
    fs::write(second_root.path().join("src/lib.rs"), source).unwrap();

    let identity =
        RepositoryIdentity::from_remote("https://GITHUB.com/CodeMind-Fixture/source-snapshot.git")
            .unwrap();
    let provider = SourceProvider::default();
    let first = provider
        .resolve_with_repository_identity(
            first_root.path(),
            "workspace-a",
            Some(identity.canonical_locator()),
        )
        .unwrap();
    let second = provider
        .resolve_with_repository_identity(
            second_root.path(),
            "workspace-a",
            Some(identity.canonical_locator()),
        )
        .unwrap();

    assert_ne!(first.root, second.root);
    assert_eq!(first.context.repository_id, identity.repository_id());
    assert_eq!(first.context.repository_id, second.context.repository_id);
    assert_eq!(
        first.context.source_snapshot_id,
        second.context.source_snapshot_id
    );
    assert_eq!(first.context.graph_version, second.context.graph_version);

    let first_graph = CoieEngine.extract(&first).unwrap();
    let second_graph = CoieEngine.extract(&second).unwrap();
    graph_builder::validate_detailed(&first_graph).unwrap();
    graph_builder::validate_detailed(&second_graph).unwrap();
}


fn query_context(workspace_id: &str) -> RepositoryContext {
    RepositoryContext {
        repository_id: "repo".into(),
        workspace_id: workspace_id.into(),
        source_snapshot_id: "snapshot".into(),
        graph_version: "v1".into(),
    }
}

fn query_edge(
    context: &RepositoryContext,
    id: &str,
    from: &str,
    to: &str,
) -> GraphRelationship {
    GraphRelationship {
        id: id.into(),
        kind: RelationshipKind::Calls,
        from: from.into(),
        to: to.into(),
        context: context.clone(),
        resolution: ResolutionStatus::Resolved,
        evidence: None,
    }
}

#[test]
fn bounded_relationship_query_reports_truncation_and_excludes_foreign_context() {
    let requested = query_context("workspace-a");
    let foreign = query_context("workspace-b");
    let relationships = vec![
        query_edge(&requested, "a-1", "root", "child"),
        query_edge(&requested, "a-2", "child", "leaf"),
        query_edge(&foreign, "b-1", "root", "foreign"),
    ];

    let result = relationships::bounded_for_context(&relationships, &requested, 1);

    assert_eq!(result.data.len(), 1);
    assert_eq!(result.data[0].id, "a-1");
    assert_eq!(result.completeness, codemind_engine::contract::result::Completeness::Truncated);
    assert_eq!(result.outcome, codemind_engine::contract::result::Outcome::Partial);
    assert!(result.limitations.iter().any(|item| item.contains("truncated")));
    assert!(
        result.diagnostics.iter().any(|item| item.contains("incompatible context")),
        "foreign-context exclusion must be visible in diagnostics"
    );
    assert!(result.data.iter().all(|edge| edge.context == requested));
}

#[test]
fn bounded_outgoing_respects_zero_and_positive_depth_after_context_filtering() {
    let requested = query_context("workspace-a");
    let foreign = query_context("workspace-b");
    let relationships = vec![
        query_edge(&requested, "a-1", "root", "child"),
        query_edge(&requested, "a-2", "child", "leaf"),
        query_edge(&foreign, "b-1", "root", "foreign"),
    ];
    let scoped = relationships::bounded_for_context(&relationships, &requested, 10);
    assert!(scoped.data.iter().all(|edge| edge.context == requested));

    assert!(
        paths::bounded_outgoing(&scoped.data, "root", 0).is_empty(),
        "depth zero must not traverse outgoing relationships"
    );

    let depth_one = paths::bounded_outgoing(&scoped.data, "root", 1);
    assert_eq!(depth_one.len(), 1);
    assert_eq!(depth_one[0].id, "a-1");

    let depth_two = paths::bounded_outgoing(&scoped.data, "root", 2);
    assert_eq!(
        depth_two.iter().map(|edge| edge.id.as_str()).collect::<HashSet<_>>(),
        HashSet::from(["a-1", "a-2"])
    );
    assert!(
        depth_two.iter().all(|edge| edge.context == requested),
        "path traversal must never return relationships from another workspace"
    );
}
