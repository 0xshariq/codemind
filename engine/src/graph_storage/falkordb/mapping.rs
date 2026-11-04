use crate::engines::coie::model::RelationshipKind;
use crate::engines::history::model::HistoryRelationshipKind;
use crate::engines::tie::model::TestRelationshipKind;

pub fn code_relationship_label(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::HasSnapshot => "HAS_SNAPSHOT",
        RelationshipKind::Contains => "CONTAINS",
        RelationshipKind::Defines => "DEFINES",
        RelationshipKind::Imports => "IMPORTS",
        RelationshipKind::Calls => "CALLS",
    }
}

pub fn test_relationship_label(kind: &TestRelationshipKind) -> &'static str {
    match kind {
        TestRelationshipKind::Contains => "CONTAINS",
        TestRelationshipKind::UsesFixture => "USES_FIXTURE",
        TestRelationshipKind::DependsOn => "DEPENDS_ON",
        TestRelationshipKind::DeclaredBy => "DECLARED_BY",
        TestRelationshipKind::Targets => "TARGETS",
        TestRelationshipKind::HasCase => "HAS_CASE",
        TestRelationshipKind::SetupBy => "SETUP_BY",
        TestRelationshipKind::TeardownBy => "TEARDOWN_BY",
    }
}

pub fn history_relationship_label(kind: &HistoryRelationshipKind) -> &'static str {
    match kind {
        HistoryRelationshipKind::ContainsCommit => "CONTAINS_COMMIT",
        HistoryRelationshipKind::ParentCommit => "PARENT_COMMIT",
        HistoryRelationshipKind::RecordsChange => "RECORDS_CHANGE",
        HistoryRelationshipKind::ChangesPath => "CHANGES_PATH",
        HistoryRelationshipKind::ContainsPath => "CONTAINS_PATH",
    }
}
