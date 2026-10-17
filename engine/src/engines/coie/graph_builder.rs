use super::model::{CodeGraph, EntityKind, RelationshipKind, ResolutionStatus};
use crate::source::limits;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub fn validate(graph: &CodeGraph) -> bool {
    validate_detailed(graph).is_ok()
}

pub fn validate_detailed(graph: &CodeGraph) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if graph.context.validate().is_err() {
        errors.push("graph context is invalid".to_owned());
    }
    let mut entity_ids = HashSet::new();
    let mut entities = HashMap::new();
    for entity in &graph.entities {
        if entity.id.trim().is_empty() || !entity_ids.insert(entity.id.clone()) {
            errors.push(format!("duplicate or empty entity id: {}", entity.id));
        }
        entities.insert(entity.id.clone(), entity);
        if entity.context != graph.context {
            errors.push(format!("entity context mismatch: {}", entity.id));
        }
        if entity.name.trim().is_empty() || entity.path.trim().is_empty() {
            errors.push(format!("invalid entity identity: {}", entity.id));
        }
        if !limits::is_safe_relative(Path::new(&entity.path)) {
            errors.push(format!("unsafe entity path: {}", entity.id));
        }
        if entity.line == 0 {
            errors.push(format!("invalid entity source line: {}", entity.id));
        }
        if let Some(range) = entity.range {
            if range.start_line == 0
                || range.end_line < range.start_line
                || range.start_byte > range.end_byte
            {
                errors.push(format!("invalid entity source range: {}", entity.id));
            }
            if range.start_line > entity.line {
                errors.push(format!(
                    "entity line precedes its source range: {}",
                    entity.id
                ));
            }
        }
        if let Some(evidence) = &entity.evidence
            && (evidence.snapshot_id != graph.context.source_snapshot_id
                || evidence.path != entity.path
                || evidence.start_line == 0
                || evidence.end_line < evidence.start_line
                || !limits::is_safe_relative(Path::new(&evidence.path)))
        {
            errors.push(format!("invalid entity evidence: {}", entity.id));
        }
        if matches!(entity.resolution, ResolutionStatus::Resolved)
            && entity.kind == EntityKind::Symbol
            && entity.evidence.is_none()
        {
            errors.push(format!(
                "resolved symbol has no source evidence: {}",
                entity.id
            ));
        }
    }
    let mut relationship_ids = HashSet::new();
    for relationship in &graph.relationships {
        if relationship.id.trim().is_empty() || !relationship_ids.insert(relationship.id.clone()) {
            errors.push(format!(
                "duplicate or empty relationship id: {}",
                relationship.id
            ));
        }
        if relationship.context != graph.context {
            errors.push(format!(
                "relationship context mismatch: {}",
                relationship.id
            ));
        }
        let from = entities.get(&relationship.from);
        let to = entities.get(&relationship.to);
        match (from, to) {
            (Some(from), Some(to)) => {
                if !legal_relationship(&relationship.kind, &from.kind, &to.kind) {
                    errors.push(format!(
                        "illegal {:?} relationship endpoints: {} -> {}",
                        relationship.kind, relationship.from, relationship.to
                    ));
                }
            }
            _ => errors.push(format!(
                "relationship {} has an unknown endpoint",
                relationship.id
            )),
        }
        if let Some(evidence) = &relationship.evidence
            && (evidence.snapshot_id != graph.context.source_snapshot_id
                || evidence.start_line == 0
                || evidence.end_line < evidence.start_line
                || !limits::is_safe_relative(Path::new(&evidence.path)))
        {
            errors.push(format!(
                "invalid relationship evidence: {}",
                relationship.id
            ));
        }
        if matches!(relationship.resolution, ResolutionStatus::Resolved)
            && relationship.evidence.is_none()
            && matches!(
                relationship.kind,
                RelationshipKind::Calls | RelationshipKind::Imports
            )
        {
            errors.push(format!(
                "resolved {:?} relationship has no source evidence: {}",
                relationship.kind, relationship.id
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn legal_relationship(kind: &RelationshipKind, from: &EntityKind, to: &EntityKind) -> bool {
    match kind {
        RelationshipKind::Contains => matches!(
            (from, to),
            (EntityKind::Repository, EntityKind::RepositorySnapshot)
                | (EntityKind::RepositorySnapshot, EntityKind::Directory)
                | (EntityKind::RepositorySnapshot, EntityKind::File)
                | (EntityKind::Directory, EntityKind::Directory)
                | (EntityKind::Directory, EntityKind::File)
                | (EntityKind::Package, EntityKind::Module)
                | (EntityKind::Module, EntityKind::Symbol)
        ),
        RelationshipKind::Defines => matches!(
            (from, to),
            (EntityKind::File, EntityKind::Symbol)
                | (EntityKind::Module, EntityKind::Symbol)
                | (EntityKind::Package, EntityKind::Symbol)
        ),
        RelationshipKind::Imports => matches!(
            (from, to),
            (EntityKind::File, EntityKind::File)
                | (EntityKind::File, EntityKind::Module)
                | (EntityKind::Module, EntityKind::Module)
        ),
        RelationshipKind::Calls => matches!((from, to), (EntityKind::Symbol, EntityKind::Symbol)),
    }
}

#[cfg(test)]
mod tests {
    use super::validate;
    use crate::engines::coie::model::{CodeGraph, EntityKind, GraphEntity, ResolutionStatus};
    use crate::source::RepositoryContext;

    #[test]
    fn graph_validation_rejects_context_mismatch_and_accepts_repository_snapshot_nodes() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let graph = CodeGraph {
            context: context.clone(),
            entities: vec![
                GraphEntity {
                    id: "repo".into(),
                    kind: EntityKind::Repository,
                    name: "repo".into(),
                    path: "repo".into(),
                    line: 1,
                    context: context.clone(),
                    language: None,
                    native_kind: Some("repository".into()),
                    range: None,
                    resolution: ResolutionStatus::Resolved,
                    evidence: None,
                },
                GraphEntity {
                    id: "snapshot".into(),
                    kind: EntityKind::RepositorySnapshot,
                    name: "snapshot".into(),
                    path: "snapshot".into(),
                    line: 1,
                    context: context.clone(),
                    language: None,
                    native_kind: Some("repository_snapshot".into()),
                    range: None,
                    resolution: ResolutionStatus::Resolved,
                    evidence: None,
                },
            ],
            relationships: Vec::new(),
            diagnostics: Vec::new(),
        };
        assert!(validate(&graph));
    }
}
