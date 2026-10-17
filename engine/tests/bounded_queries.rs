use codemind_engine::{
    contract::result::{Completeness, Outcome, serialized_size},
    engines::coie::{
        model::{GraphRelationship, RelationshipKind},
        queries::{paths, relationships},
    },
    source::RepositoryContext,
};

fn context(workspace_id: &str) -> RepositoryContext {
    RepositoryContext {
        repository_id: "repo".to_owned(),
        workspace_id: workspace_id.to_owned(),
        source_snapshot_id: "snapshot".to_owned(),
        graph_version: "day-2".to_owned(),
    }
}

fn relationship(context: RepositoryContext, id: &str, from: &str) -> GraphRelationship {
    GraphRelationship {
        id: id.to_owned(),
        kind: RelationshipKind::Calls,
        from: from.to_owned(),
        to: "target".to_owned(),
        context,
        resolution: codemind_engine::engines::coie::model::ResolutionStatus::Resolved,
        evidence: Some(codemind_engine::engines::coie::model::SourceEvidence {
            path: "src/lib.rs".to_owned(),
            start_line: 1,
            end_line: 1,
            snapshot_id: "snapshot".to_owned(),
            provenance: "test".to_owned(),
        }),
    }
}

#[test]
fn bounded_relationships_preserve_context_and_report_truncation() {
    let requested = context("workspace-a");
    let other = context("workspace-b");
    let facts = vec![
        relationship(requested.clone(), "one", "source"),
        relationship(requested.clone(), "two", "source"),
        relationship(other, "foreign", "source"),
    ];

    let result = relationships::bounded_for_context(&facts, &requested, 1);
    assert_eq!(result.outcome, Outcome::Partial);
    assert_eq!(result.completeness, Completeness::Truncated);
    assert_eq!(result.data.len(), 1);
    assert_eq!(result.context, requested);
    assert_eq!(result.diagnostics.len(), 1);
    assert!(serialized_size(&result).unwrap() > 0);
}

#[test]
fn path_depth_zero_is_empty_and_positive_depth_is_bounded() {
    let context = context("workspace-a");
    let facts = vec![
        relationship(context.clone(), "one", "source"),
        relationship(context, "two", "source"),
    ];
    assert!(paths::bounded_outgoing(&facts, "source", 0).is_empty());
    assert_eq!(paths::bounded_outgoing(&facts, "source", 1).len(), 2);
}
