use crate::engines::{
    chie::model::{ImpactClassification, ImpactPath, traversable},
    coie::model::{GraphRelationship, RelationshipKind},
};
use std::collections::{HashSet, VecDeque};

pub fn traverse(
    relationships: &[GraphRelationship],
    selected_entity_id: &str,
    max_depth: usize,
) -> Vec<ImpactPath> {
    let mut queue = VecDeque::from([(selected_entity_id.to_owned(), 0usize, Vec::new())]);
    let mut visited = HashSet::from([selected_entity_id.to_owned()]);
    let mut paths = Vec::new();
    while let Some((entity, depth, path)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        for relationship in relationships.iter().filter(|relationship| {
            relationship.from == entity && traversable(relationship.kind.clone())
        }) {
            if !visited.insert(relationship.to.clone()) {
                continue;
            }
            let mut next_path = path.clone();
            next_path.push(relationship.clone());
            let next_depth = depth + 1;
            paths.push(ImpactPath {
                entity_id: relationship.to.clone(),
                classification: if next_depth == 1 {
                    ImpactClassification::Direct
                } else {
                    ImpactClassification::Indirect
                },
                depth: next_depth,
                relationships: next_path.clone(),
            });
            queue.push_back((relationship.to.clone(), next_depth, next_path));
        }
    }
    paths
}

pub fn relationship_is_supported(kind: RelationshipKind) -> bool {
    traversable(kind)
}
