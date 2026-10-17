pub mod evidence;
pub mod target_mapping;

#[cfg(test)]
mod integration_tests {
    use super::target_mapping::resolve_target;
    use crate::engines::coie::model::{
        CodeGraph, EntityKind, GraphEntity, GraphRelationship, RelationshipKind, ResolutionStatus,
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

    fn file(id: &str, path: &str) -> GraphEntity {
        GraphEntity {
            id: id.into(),
            kind: EntityKind::File,
            name: path.into(),
            path: path.into(),
            line: 1,
            context: context(),
            language: None,
            native_kind: Some("file".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        }
    }

    #[test]
    fn target_mapping_resolves_across_a_canonical_import_edge() {
        let source = file("file-a", "src/a.rs");
        let target_file = file("file-b", "src/b.rs");
        let target = GraphEntity {
            id: "symbol-b".into(),
            kind: EntityKind::Symbol,
            name: "helper".into(),
            path: "src/b.rs".into(),
            line: 1,
            context: context(),
            language: None,
            native_kind: Some("function".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: Some(crate::engines::coie::model::SourceEvidence {
                path: "src/b.rs".into(),
                start_line: 1,
                end_line: 1,
                snapshot_id: "snapshot".into(),
                provenance: "test".into(),
            }),
        };
        let graph = CodeGraph {
            context: context(),
            entities: vec![source, target_file, target],
            relationships: vec![GraphRelationship {
                id: "import".into(),
                kind: RelationshipKind::Imports,
                from: "file-a".into(),
                to: "file-b".into(),
                context: context(),
                resolution: ResolutionStatus::Resolved,
                evidence: Some(crate::engines::coie::model::SourceEvidence {
                    path: "src/a.rs".into(),
                    start_line: 1,
                    end_line: 1,
                    snapshot_id: "snapshot".into(),
                    provenance: "test".into(),
                }),
            }],
            diagnostics: Vec::new(),
        };
        let (id, resolution) = resolve_target(&graph, "src/a.rs", "helper", 1);
        assert_eq!(id.as_deref(), Some("symbol-b"));
        assert_eq!(
            resolution,
            crate::engines::tie::model::TestResolution::Resolved
        );
    }
}
