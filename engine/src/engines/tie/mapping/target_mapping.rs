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
    let same_file_candidate = match same_file.as_slice() {
        [entity] => Some(*entity),
        [] => None,
        _ => return (None, TestResolution::Ambiguous),
    };

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
            [entity] if same_file_candidate.is_none() => {
                return (Some(entity.id.clone()), TestResolution::Resolved);
            }
            [entity] => {
                // The graph does not retain lexical binding/scope information
                // for this test call. Do not guess between a local symbol and
                // an imported symbol with the same name.
                let _ = entity;
                return (None, TestResolution::Ambiguous);
            }
            [] => {
                if same_file_candidate.is_none() {
                    return (None, TestResolution::Unresolved);
                }
            }
            _ => return (None, TestResolution::Ambiguous),
        }
    }

    same_file_candidate
        .map(|entity| (Some(entity.id.clone()), TestResolution::Resolved))
        .unwrap_or((None, TestResolution::Unresolved))
}

fn resolve_declaration_target(
    code_graph: &CodeGraph,
    path: &str,
    name: &str,
    line: u32,
) -> (Option<String>, TestResolution) {
    let symbols: Vec<_> = code_graph
        .entities
        .iter()
        .filter(|entity| {
            entity.kind == EntityKind::Symbol
                && entity.path == path
                && entity.name == name
                && entity.context == code_graph.context
        })
        .collect();
    if symbols.len() == 1 {
        return (Some(symbols[0].id.clone()), TestResolution::Resolved);
    }
    if symbols.len() > 1 {
        return (None, TestResolution::Ambiguous);
    }

    let files: Vec<_> = code_graph
        .entities
        .iter()
        .filter(|entity| {
            entity.kind == EntityKind::File
                && entity.path == path
                && entity.context == code_graph.context
        })
        .collect();
    if files.len() == 1 && line > 0 {
        return (Some(files[0].id.clone()), TestResolution::Resolved);
    }
    if files.len() > 1 {
        return (None, TestResolution::Ambiguous);
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
                    let line = relationship
                        .evidence
                        .as_ref()
                        .map(|evidence| evidence.start_line)
                        .unwrap_or(0);
                    let name = relationship.target_symbol_name.clone().unwrap_or_default();
                    let (entity_id, resolution) = resolve_declaration_target(
                        code_graph,
                        &relationship.evidence_path,
                        &name,
                        line,
                    );
                    relationship.entity_id = entity_id;
                    relationship.resolution = resolution;
                    relationship.target_kind = crate::engines::tie::model::TestTargetKind::Code;
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
                        relationship.target_kind = crate::engines::tie::model::TestTargetKind::Code;
                    }
                }
                TestRelationshipKind::Contains
                | TestRelationshipKind::UsesFixture
                | TestRelationshipKind::DependsOn
                | TestRelationshipKind::HasCase
                | TestRelationshipKind::SetupBy
                | TestRelationshipKind::TeardownBy => {}
            }
            relationship
        })
        .collect();
    (tests, mapped)
}

pub fn supported(kind: TestRelationshipKind) -> bool {
    matches!(
        kind,
        TestRelationshipKind::Contains
            | TestRelationshipKind::UsesFixture
            | TestRelationshipKind::DependsOn
            | TestRelationshipKind::DeclaredBy
            | TestRelationshipKind::Targets
            | TestRelationshipKind::HasCase
    )
}
