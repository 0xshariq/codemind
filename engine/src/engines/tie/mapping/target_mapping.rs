use crate::engines::coie::model::{CodeGraph, EntityKind, RelationshipKind, ResolutionStatus};
use crate::engines::tie::model::{
    TestEntity, TestRelationship, TestRelationshipKind, TestResolution,
};

pub fn resolve_target(
    code_graph: &CodeGraph,
    path: &str,
    name: &str,
    _line: u32,
) -> (Option<String>, TestResolution) {
    let short = name.rsplit('.').next().unwrap_or(name);
    let same_file: Vec<_> = code_graph
        .entities
        .iter()
        .filter(|entity| {
            entity.kind == EntityKind::Symbol
                && entity.path == path
                && entity.name == short
                && entity.context == code_graph.context
        })
        .collect();
    match same_file.as_slice() {
        [entity] => return (Some(entity.id.clone()), TestResolution::Resolved),
        [] => {}
        _ => return (None, TestResolution::Ambiguous),
    }

    let imported_file_ids: Vec<&str> = code_graph
        .entities
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::File
                && entity.path == path
                && entity.context == code_graph.context
        })
        .map(|file| {
            code_graph
                .relationships
                .iter()
                .filter(|relationship| {
                    relationship.from == file.id
                        && relationship.kind == RelationshipKind::Imports
                        && relationship.resolution == ResolutionStatus::Resolved
                        && relationship.context == code_graph.context
                })
                .map(|relationship| relationship.to.as_str())
                .collect()
        })
        .unwrap_or_default();

    if !imported_file_ids.is_empty() {
        let imported_targets: Vec<_> = code_graph
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Symbol
                    && entity.name == short
                    && entity.context == code_graph.context
                    && imported_file_ids.iter().any(|file_id| {
                        code_graph.entities.iter().any(|file| {
                            file.id == *file_id
                                && file.kind == EntityKind::File
                                && file.context == code_graph.context
                                && file.path == entity.path
                        })
                    })
            })
            .collect();
        match imported_targets.as_slice() {
            [entity] => return (Some(entity.id.clone()), TestResolution::Resolved),
            [] => return (None, TestResolution::Unresolved),
            _ => return (None, TestResolution::Ambiguous),
        }
    }

    (None, TestResolution::Unresolved)
}

pub fn resolve_graph(
    code_graph: &CodeGraph,
    tests: Vec<TestEntity>,
    relationships: Vec<TestRelationship>,
) -> (Vec<TestEntity>, Vec<TestRelationship>) {
    let mapped = relationships
        .into_iter()
        .map(|mut relationship| {
            match relationship.kind {
                TestRelationshipKind::DeclaredBy => {
                    if let Some(entity) = code_graph.entities.iter().find(|entity| {
                        entity.kind == EntityKind::File
                            && entity.path == relationship.evidence_path
                            && entity.context == relationship.context
                    }) {
                        relationship.entity_id = Some(entity.id.clone());
                        relationship.resolution = TestResolution::Resolved;
                    }
                }
                TestRelationshipKind::Targets => {
                    if let Some(name) = relationship.target_symbol_name.clone() {
                        let line = relationship
                            .evidence
                            .as_ref()
                            .map(|evidence| evidence.start_line)
                            .unwrap_or(0);
                        let evidence_path = relationship
                            .evidence
                            .as_ref()
                            .map(|evidence| evidence.path.as_str())
                            .unwrap_or("");
                        let (entity_id, resolution) =
                            resolve_target(code_graph, evidence_path, &name, line);
                        relationship.entity_id = entity_id;
                        relationship.resolution = resolution;
                    }
                }
            }
            relationship
        })
        .collect();
    (tests, mapped)
}

pub fn supported(kind: TestRelationshipKind) -> bool {
    matches!(
        kind,
        TestRelationshipKind::DeclaredBy | TestRelationshipKind::Targets
    )
}

#[cfg(test)]
mod tests {
    use super::resolve_target;
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

    #[test]
    fn target_resolution_prefers_same_file_and_rejects_ambiguous_imports() {
        let same = GraphEntity {
            id: "same".into(),
            kind: EntityKind::Symbol,
            name: "helper".into(),
            path: "src/a.rs".into(),
            line: 1,
            context: context(),
            language: None,
            native_kind: Some("function".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        };
        let relationship = GraphRelationship {
            id: "self-call".into(),
            kind: RelationshipKind::Calls,
            from: "same".into(),
            to: "same".into(),
            context: context(),
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        };
        let graph = CodeGraph {
            context: context(),
            entities: vec![same],
            relationships: vec![relationship],
            diagnostics: Vec::new(),
        };
        let (id, status) = resolve_target(&graph, "src/a.rs", "helper", 1);
        assert_eq!(id.as_deref(), Some("same"));
        assert_eq!(status, crate::engines::tie::model::TestResolution::Resolved);
    }
}
