use crate::engines::coie::model::GraphRelationship;

pub fn outgoing<'a>(
    relationships: &'a [GraphRelationship],
    entity_id: &str,
) -> impl Iterator<Item = &'a GraphRelationship> {
    relationships
        .iter()
        .filter(move |relationship| relationship.from == entity_id)
}
