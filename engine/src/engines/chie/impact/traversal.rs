use crate::engines::{
    chie::model::{ImpactClassification, ImpactPath, traversable},
    coie::model::{GraphRelationship, RelationshipKind},
};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalResult {
    pub paths: Vec<ImpactPath>,
    pub truncated: bool,
}

/// For a selected changed symbol, impact propagates against dependency
/// semantics: a CALLS edge is caller -> callee, so callers are reached by
/// following the edge backwards. IMPORTS is treated the same way for the
/// static dependency model.
///
/// Traversal is deliberately bounded before paths are materialized further.
/// A single duplicate edge must not multiply the search, and an entity already
/// present in a path cannot be visited again within that path.
pub fn traverse(
    relationships: &[GraphRelationship],
    selected_entity_id: &str,
    max_depth: usize,
    max_paths: usize,
) -> TraversalResult {
    if selected_entity_id.trim().is_empty() || max_depth == 0 || max_paths == 0 {
        return TraversalResult {
            paths: Vec::new(),
            truncated: false,
        };
    }

    let max_work = max_paths
        .saturating_mul(max_depth.saturating_add(1))
        .max(max_paths);

    let mut adjacency: HashMap<String, Vec<&GraphRelationship>> = HashMap::new();
    let mut edge_ids = HashSet::new();

    for relationship in relationships {
        if !traversable(&relationship.kind)
            || !matches!(
                relationship.resolution,
                crate::engines::coie::model::ResolutionStatus::Resolved
            )
            || !edge_ids.insert(relationship.id.clone())
        {
            continue;
        }

        adjacency
            .entry(relationship.to.clone())
            .or_default()
            .push(relationship);
    }

    let mut queue = VecDeque::from([(
        selected_entity_id.to_owned(),
        0usize,
        Vec::<GraphRelationship>::new(),
        HashSet::<String>::from([selected_entity_id.to_owned()]),
    )]);

    let mut paths = Vec::new();
    let mut work = 0usize;
    let mut truncated = false;

    while let Some((entity, depth, path, visited_entities)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        let Some(incoming) = adjacency.get(&entity) else {
            continue;
        };

        for relationship in incoming {
            work = work.saturating_add(1);

            if work > max_work {
                truncated = true;
                break;
            }

            if visited_entities.contains(&relationship.from) {
                continue;
            }

            if paths.len() >= max_paths {
                truncated = true;
                break;
            }

            let mut next_path = path.clone();
            next_path.push((*relationship).clone());

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

            let mut next_visited = visited_entities.clone();
            next_visited.insert(relationship.from.clone());

            queue.push_back((
                relationship.from.clone(),
                next_depth,
                next_path,
                next_visited,
            ));
        }

        if truncated {
            break;
        }
    }

    TraversalResult { paths, truncated }
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

        let edge = |id: &str, from: &str, to: &str, resolution| -> GraphRelationship {
            GraphRelationship {
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
            }
        };

        let result = traverse(
            &[
                edge("a-b", "a", "b", ResolutionStatus::Resolved),
                edge("b-c", "b", "c", ResolutionStatus::Resolved),
                edge("x-c", "x", "c", ResolutionStatus::Unresolved),
            ],
            "c",
            2,
            10,
        );

        assert_eq!(result.paths.len(), 2);
        assert!(result.paths.iter().any(|path| path.entity_id == "b"));
        assert!(result.paths.iter().any(|path| path.entity_id == "a"));
        assert!(!result.truncated);
    }

    #[test]
    fn traversal_rejects_cycles_and_bounds_work_before_queue_growth() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };

        let edge = |id: &str, from: &str, to: &str| GraphRelationship {
            id: id.into(),
            kind: RelationshipKind::Calls,
            from: from.into(),
            to: to.into(),
            context: context.clone(),
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        };

        let result = traverse(
            &[
                edge("a-b", "a", "b"),
                edge("b-a", "b", "a"),
                edge("a-c", "a", "c"),
                edge("a-c-duplicate", "a", "c"),
            ],
            "c",
            8,
            1,
        );

        assert_eq!(result.paths.len(), 1);
        assert!(result.truncated);
        assert!(
            result.paths[0]
                .relationships
                .iter()
                .all(|relationship| relationship.from != "c")
        );
    }
}
