//! Logical domain labels shared inside a workspace's physical FalkorDB graph.
//!
//! Physical graph routing is deliberately not represented by labels or Cypher
//! properties here. The adapter resolves the graph from the validated
//! `workspace_id`; repository, snapshot, and projection-version properties remain
//! explicit filters inside the selected workspace graph.

/// Version of the physical FalkorDB representation and required access paths.
///
/// This is intentionally independent from `GRAPH_PROJECTION_VERSION` and from
/// repository metadata schema versions. Bump it when persisted storage shape or
/// required index/compatibility semantics change.
pub const GRAPH_STORAGE_SCHEMA_VERSION: u32 = 5;
pub const STORAGE_SCHEMA_MARKER_ID: &str = "codemind-storage-schema";
pub const ACTIVE_GENERATION_LABEL: &str = "CodeMindActiveGeneration";
pub const ACTIVE_GENERATION_ID: &str = "codemind-active-generation";

/// Required node range indexes. FalkorDB indexes are single-property access
/// paths, so scope and stable-ID fields are indexed independently.
pub const REQUIRED_NODE_INDEXES: &[(&str, &str)] = &[
    ("CodeEntity", "id"),
    ("CodeEntity", "workspace_id"),
    ("CodeEntity", "repository_id"),
    ("CodeEntity", "snapshot_id"),
    ("CodeEntity", "graph_version"),
    ("TestEntity", "id"),
    ("TestEntity", "workspace_id"),
    ("TestEntity", "repository_id"),
    ("TestEntity", "snapshot_id"),
    ("TestEntity", "graph_version"),
    ("HistoryRepository", "id"),
    ("HistoryRepository", "workspace_id"),
    ("HistoryRepository", "repository_id"),
    ("HistoryRepository", "generation_id"),
    ("HistoryRepository", "logical_id"),
    ("HistoryCommit", "id"),
    ("HistoryCommit", "commit_id"),
    ("HistoryCommit", "workspace_id"),
    ("HistoryCommit", "repository_id"),
    ("HistoryCommit", "generation_id"),
    ("HistoryCommit", "logical_id"),
    ("HistoryDirectory", "id"),
    ("HistoryDirectory", "generation_id"),
    ("HistoryDirectory", "logical_id"),
    ("HistoryFile", "id"),
    ("HistoryFile", "generation_id"),
    ("HistoryFile", "logical_id"),
    ("HistoryChange", "id"),
    ("HistoryChange", "generation_id"),
    ("HistoryChange", "logical_id"),
    ("HistoryGeneration", "id"),
    ("HistoryGeneration", "repository_generation_id"),
    (ACTIVE_GENERATION_LABEL, "history_generation_id"),
    ("ChangeSet", "id"),
    ("ChangeRecord", "id"),
    ("AnalysisRun", "id"),
    ("ImpactAnalysis", "id"),
    ("RiskAssessment", "id"),
    ("Evidence", "id"),
    ("OperationRecord", "id"),
    ("OperationEvent", "id"),
    ("MemoryRecord", "id"),
    ("HandoffRecord", "id"),
    ("ReviewContextRecord", "id"),
    ("ConversationSession", "id"),
    ("ReviewEpisode", "id"),
    ("ConversationTurn", "id"),
    ("ReviewFinding", "id"),
    (ACTIVE_GENERATION_LABEL, "id"),
];

pub const REQUIRED_EDGE_INDEXES: &[(&str, &str)] = &[
    ("HAS_SNAPSHOT", "id"),
    ("CONTAINS", "id"),
    ("DEFINES", "id"),
    ("IMPORTS", "id"),
    ("CALLS", "id"),
    ("USES_FIXTURE", "id"),
    ("DEPENDS_ON", "id"),
    ("DECLARED_BY", "id"),
    ("TARGETS", "id"),
    ("HAS_CASE", "id"),
    ("SETUP_BY", "id"),
    ("TEARDOWN_BY", "id"),
    ("CONTAINS_COMMIT", "id"),
    ("PARENT_COMMIT", "id"),
    ("RECORDS_CHANGE", "id"),
    ("CHANGES_PATH", "id"),
    ("CONTAINS_PATH", "id"),
    ("CONTAINS_COMMIT", "generation_id"),
    ("PARENT_COMMIT", "generation_id"),
    ("RECORDS_CHANGE", "generation_id"),
    ("CHANGES_PATH", "generation_id"),
    ("CONTAINS_PATH", "generation_id"),
    ("CONTAINS_CHANGE", "id"),
    ("ANALYZES", "id"),
    ("PRODUCES", "id"),
    ("AFFECTS", "id"),
    ("HAS_RISK", "id"),
    ("RECOMMENDS", "id"),
    ("SUPPORTED_BY", "id"),
];

pub const CODE_ENTITY_LABEL: &str = "CodeEntity";
pub const TEST_ENTITY_LABEL: &str = "TestEntity";
pub const CHANGE_ENTITY_LABELS: &[&str] = &[
    "ChangeSet",
    "ChangeRecord",
    "AnalysisRun",
    "ImpactAnalysis",
    "RiskAssessment",
    "Evidence",
];

pub const CODE_RELATIONSHIP_LABELS: &[&str] =
    &["HAS_SNAPSHOT", "CONTAINS", "DEFINES", "IMPORTS", "CALLS"];

pub const TEST_RELATIONSHIP_LABELS: &[&str] = &[
    "CONTAINS",
    "USES_FIXTURE",
    "DEPENDS_ON",
    "DECLARED_BY",
    "TARGETS",
    "HAS_CASE",
    "SETUP_BY",
    "TEARDOWN_BY",
];
pub const HISTORY_ENTITY_LABELS: &[&str] = &[
    "HistoryRepository",
    "HistoryCommit",
    "HistoryDirectory",
    "HistoryFile",
    "HistoryChange",
];
pub const HISTORY_RELATIONSHIP_LABELS: &[&str] = &[
    "CONTAINS_COMMIT",
    "PARENT_COMMIT",
    "RECORDS_CHANGE",
    "CHANGES_PATH",
    "CONTAINS_PATH",
];

pub const CHANGE_RELATIONSHIP_LABELS: &[&str] = &[
    "CONTAINS_CHANGE",
    "ANALYZES",
    "PRODUCES",
    "AFFECTS",
    "HAS_RISK",
    "RECOMMENDS",
    "SUPPORTED_BY",
];

pub fn relationship_type(kind: &str) -> Option<&'static str> {
    match kind {
        "HasSnapshot" => Some("HAS_SNAPSHOT"),
        "Contains" => Some("CONTAINS"),
        "Defines" => Some("DEFINES"),
        "Imports" => Some("IMPORTS"),
        "Calls" => Some("CALLS"),
        "DeclaredBy" => Some("DECLARED_BY"),
        "Targets" => Some("TARGETS"),
        "UsesFixture" => Some("USES_FIXTURE"),
        "DependsOn" => Some("DEPENDS_ON"),
        "HasCase" => Some("HAS_CASE"),
        "SetupBy" => Some("SETUP_BY"),
        "TeardownBy" => Some("TEARDOWN_BY"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ACTIVE_GENERATION_ID, ACTIVE_GENERATION_LABEL, GRAPH_STORAGE_SCHEMA_VERSION,
        REQUIRED_EDGE_INDEXES, REQUIRED_NODE_INDEXES, STORAGE_SCHEMA_MARKER_ID, relationship_type,
    };

    #[test]
    fn storage_schema_contract_is_explicit_and_nonzero() {
        const { assert!(GRAPH_STORAGE_SCHEMA_VERSION > 0) };
        assert!(!STORAGE_SCHEMA_MARKER_ID.is_empty());
        assert_eq!(ACTIVE_GENERATION_ID, "codemind-active-generation");
        assert!(!ACTIVE_GENERATION_LABEL.is_empty());
        assert!(!REQUIRED_NODE_INDEXES.is_empty());
        assert!(!REQUIRED_EDGE_INDEXES.is_empty());
        assert!(REQUIRED_NODE_INDEXES.contains(&("CodeEntity", "id")));
        assert!(REQUIRED_NODE_INDEXES.contains(&("CodeEntity", "workspace_id")));
        assert!(REQUIRED_NODE_INDEXES.contains(&("HistoryCommit", "commit_id")));
        assert!(REQUIRED_NODE_INDEXES.contains(&("HistoryGeneration", "id")));
        assert!(REQUIRED_EDGE_INDEXES.contains(&("HAS_SNAPSHOT", "id")));
        assert!(REQUIRED_EDGE_INDEXES.contains(&("TARGETS", "id")));
    }

    #[test]
    fn schema_rejects_unknown_relationship_labels() {
        assert_eq!(relationship_type("HasSnapshot"), Some("HAS_SNAPSHOT"));
        assert_eq!(relationship_type("Contains"), Some("CONTAINS"));
        assert_eq!(relationship_type("Targets"), Some("TARGETS"));
        assert_eq!(relationship_type("UsesFixture"), Some("USES_FIXTURE"));
        assert_eq!(relationship_type("DependsOn"), Some("DEPENDS_ON"));
        assert_eq!(relationship_type("HasCase"), Some("HAS_CASE"));
        assert_eq!(relationship_type("AFFECTS"), None);
    }
}
