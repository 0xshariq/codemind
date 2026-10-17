pub mod history;
pub mod paths;
pub mod relationships;
pub mod search;
pub mod symbols;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::contract::result::Outcome;
    use crate::engines::coie::model::{
        EntityKind, GraphEntity, GraphRelationship, RelationshipKind, ResolutionStatus,
    };
    use crate::source::RepositoryContext;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    fn entity(id: &str, name: &str) -> GraphEntity {
        GraphEntity {
            id: id.into(),
            kind: EntityKind::Symbol,
            name: name.into(),
            path: "src/lib.rs".into(),
            line: 1,
            context: context(),
            language: None,
            native_kind: Some("function".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: Some(crate::engines::coie::model::SourceEvidence {
                path: "src/lib.rs".into(),
                start_line: 1,
                end_line: 1,
                snapshot_id: "snapshot".into(),
                provenance: "test".into(),
            }),
        }
    }

    #[test]
    fn query_module_composes_context_filtering_bounds_search_and_traversal() {
        let graph = vec![entity("a", "target"), entity("b", "target")];
        let relationship = GraphRelationship {
            id: "a-b".into(),
            kind: RelationshipKind::Calls,
            from: "a".into(),
            to: "b".into(),
            context: context(),
            resolution: ResolutionStatus::Resolved,
            evidence: Some(crate::engines::coie::model::SourceEvidence {
                path: "src/lib.rs".into(),
                start_line: 1,
                end_line: 1,
                snapshot_id: "snapshot".into(),
                provenance: "test".into(),
            }),
        };
        assert_eq!(
            symbols::bounded_symbols(&graph, &context(), 1).data.len(),
            1
        );
        assert_eq!(
            search::bounded_by_name(&graph, &context(), "target", 1)
                .data
                .len(),
            1
        );
        assert_eq!(
            relationships::bounded_for_context(std::slice::from_ref(&relationship), &context(), 10)
                .data
                .len(),
            1
        );
        assert!(matches!(
            relationships::bounded_for_context(std::slice::from_ref(&relationship), &context(), 10)
                .outcome,
            Outcome::Success
        ));
        let provenance = history::snapshot_provenance(&context());
        assert_eq!(provenance.data.source_snapshot_id, "snapshot");
        assert_eq!(
            paths::bounded_outgoing(std::slice::from_ref(&relationship), "a", 1).len(),
            1
        );
    }
}
