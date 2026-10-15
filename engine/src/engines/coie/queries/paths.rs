use crate::contract::result::MAX_RELATIONSHIPS;
use crate::engines::coie::model::GraphRelationship;

pub fn outgoing<'a>(
    relationships: &'a [GraphRelationship],
    entity_id: &str,
) -> impl Iterator<Item = &'a GraphRelationship> {
    relationships
        .iter()
        .filter(move |relationship| relationship.from == entity_id)
}

pub fn bounded_outgoing<'a>(
    relationships: &'a [GraphRelationship],
    entity_id: &str,
    max_depth: usize,
) -> Vec<&'a GraphRelationship> {
    if max_depth == 0 {
        return Vec::new();
    }
    outgoing(relationships, entity_id)
        .take(MAX_RELATIONSHIPS)
        .collect()
}
