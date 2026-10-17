use crate::engines::{
    chie::model::{ImpactClassification, ImpactPath, traversable},
    coie::model::{GraphRelationship, RelationshipKind},
};
use std::collections::VecDeque;

/// For a selected changed symbol, impact propagates against dependency
/// semantics: a CALLS edge is caller -> callee, so callers are reached by
/// following the edge backwards. IMPORTS is treated the same way for the
/// static dependency model.
pub fn traverse(
    relationships: &[GraphRelationship],
    selected_entity_id: &str,
    max_depth: usize,
) -> Vec<ImpactPath> {
    if selected_entity_id.trim().is_empty() || max_depth == 0 {
        return Vec::new();
    }

    let mut queue = VecDeque::from([(selected_entity_id.to_owned(), 0usize, Vec::new())]);
    let mut paths = Vec::new();

    while let Some((entity, depth, path)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        for relationship in relationships.iter().filter(|relationship| {
            relationship.to == entity
                && traversable(&relationship.kind)
                && matches!(
                    relationship.resolution,
                    crate::engines::coie::model::ResolutionStatus::Resolved
                )
        }) {
            let mut next_path = path.clone();
            next_path.push(relationship.clone());
            let next_depth = depth + 1;

            paths.push(ImpactPath {
                entity_id: relationship.from.clone(),
                classification: if next_depth == 1 {
                    ImpactClassification::Direct
                } else {
                    ImpactClassification::Indirect
                },
                depth: next_depth,
                relationships: next_path.clone(),
            });

            queue.push_back((relationship.from.clone(), next_depth, next_path));
        }
    }
    paths
}

pub fn relationship_is_supported(kind: RelationshipKind) -> bool {
    traversable(&kind)
}

#[cfg(test)]
mod tests {
    use super::traverse;
    use crate::engines::coie::model::{
        GraphRelationship, RelationshipKind, ResolutionStatus, SourceEvidence,
    };
    use crate::source::RepositoryContext;

    #[test]
    fn reverse_dependency_traversal_respects_depth_and_resolution() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let edge = |id: &str, from: &str, to: &str, resolution| GraphRelationship {
            id: id.into(),
            kind: RelationshipKind::Calls,
            from: from.into(),
            to: to.into(),
            context: context.clone(),
            resolution,
            evidence: Some(SourceEvidence {
                path: "src/lib.rs".into(),
                start_line: 1,
                end_line: 1,
                snapshot_id: "snapshot".into(),
                provenance: "test".into(),
            }),
        };
        let paths = traverse(
            &[
                edge("a-b", "a", "b", ResolutionStatus::Resolved),
                edge("b-c", "b", "c", ResolutionStatus::Resolved),
                edge("x-c", "x", "c", ResolutionStatus::Unresolved),
            ],
            "c",
            2,
        );
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|path| path.entity_id == "b"));
        assert!(paths.iter().any(|path| path.entity_id == "a"));
    }
}
