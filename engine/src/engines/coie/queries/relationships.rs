use crate::engines::coie::model::{GraphRelationship, RelationshipKind};

pub fn of_kind(
    relationships: &[GraphRelationship],
    kind: RelationshipKind,
) -> impl Iterator<Item = &GraphRelationship> {
    relationships
        .iter()
        .filter(move |relationship| relationship.kind == kind)
}
