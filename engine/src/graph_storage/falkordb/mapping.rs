use crate::engines::coie::model::RelationshipKind;

pub fn code_relationship_label(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Defines => "DEFINES",
        RelationshipKind::Imports => "IMPORTS",
        RelationshipKind::Calls => "CALLS",
    }
}
