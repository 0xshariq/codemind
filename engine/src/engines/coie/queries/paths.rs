use crate::{
    contract::result::MAX_PATHS,
    engines::coie::model::{GraphRelationship, RelationshipKind},
};
use std::collections::{HashSet, VecDeque};

pub fn outgoing<'a>(
    relationships: &'a [GraphRelationship],
    entity_id: &str,
) -> impl Iterator<Item = &'a GraphRelationship> {
    relationships.iter().filter(move |r| r.from == entity_id)
}

/// Bounded breadth-first traversal over an in-memory graph. This helper mirrors
/// the semantic relationship set used by CoIE and is intended for unit tests;
/// production multi-hop analysis must remain storage-backed.
pub fn bounded_outgoing<'a>(
    relationships: &'a [GraphRelationship],
    entity_id: &str,
    max_depth: usize,
) -> Vec<&'a GraphRelationship> {
    if max_depth == 0 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut queue = VecDeque::from([(entity_id.to_owned(), 0usize)]);
    let mut visited = HashSet::from([entity_id.to_owned()]);
    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        for edge in relationships
            .iter()
            .filter(|r| r.from == current && traversable(&r.kind))
        {
            if result.len() >= MAX_PATHS {
                return result;
            }
            result.push(edge);
            if visited.insert(edge.to.clone()) {
                queue.push_back((edge.to.clone(), depth + 1));
            }
        }
    }
    result
}

fn traversable(kind: &RelationshipKind) -> bool {
    matches!(kind, RelationshipKind::Calls | RelationshipKind::Imports)
}

#[cfg(test)]
mod tests {
    use super::{bounded_outgoing, outgoing};
    use crate::engines::coie::model::{
        GraphRelationship, RelationshipKind, ResolutionStatus, SourceEvidence,
    };
    use crate::source::RepositoryContext;

    #[test]
    fn path_queries_follow_only_supported_dependency_relationships() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let edge = |id: &str, kind: RelationshipKind, from: &str, to: &str| GraphRelationship {
            id: id.into(),
            kind,
            from: from.into(),
            to: to.into(),
            context: context.clone(),
            resolution: ResolutionStatus::Resolved,
            evidence: Some(SourceEvidence {
                path: "src/lib.rs".into(),
                start_line: 1,
                end_line: 1,
                snapshot_id: "snapshot".into(),
                provenance: "test".into(),
            }),
        };
        let relationships = vec![
            edge("call", RelationshipKind::Calls, "a", "b"),
            edge("define", RelationshipKind::Defines, "a", "c"),
        ];
        assert_eq!(outgoing(&relationships, "a").count(), 2);
        assert_eq!(bounded_outgoing(&relationships, "a", 1).len(), 1);
    }
}
