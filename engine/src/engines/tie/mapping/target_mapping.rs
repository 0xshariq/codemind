use super::super::model::TestRelationshipKind;

pub fn supported(kind: TestRelationshipKind) -> bool {
    matches!(
        kind,
        TestRelationshipKind::DeclaredBy | TestRelationshipKind::Targets
    )
}
