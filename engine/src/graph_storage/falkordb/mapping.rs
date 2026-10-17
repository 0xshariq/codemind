use crate::engines::coie::model::RelationshipKind;

pub fn code_relationship_label(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Contains => "CONTAINS",
        RelationshipKind::Defines => "DEFINES",
        RelationshipKind::Imports => "IMPORTS",
        RelationshipKind::Calls => "CALLS",
    }
}

#[cfg(test)]
mod tests {
    use super::code_relationship_label;
    use crate::engines::coie::model::RelationshipKind;

    #[test]
    fn maps_every_canonical_code_relationship_to_its_graph_label() {
        assert_eq!(
            code_relationship_label(&RelationshipKind::Contains),
            "CONTAINS"
        );
        assert_eq!(
            code_relationship_label(&RelationshipKind::Defines),
            "DEFINES"
        );
        assert_eq!(
            code_relationship_label(&RelationshipKind::Imports),
            "IMPORTS"
        );
        assert_eq!(code_relationship_label(&RelationshipKind::Calls), "CALLS");
    }
}
