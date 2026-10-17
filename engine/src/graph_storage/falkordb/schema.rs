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

pub const CODE_RELATIONSHIP_LABELS: &[&str] = &["CONTAINS", "DEFINES", "IMPORTS", "CALLS"];

pub const TEST_RELATIONSHIP_LABELS: &[&str] = &["DECLARED_BY", "TARGETS"];
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
        "Contains" => Some("CONTAINS"),
        "Defines" => Some("DEFINES"),
        "Imports" => Some("IMPORTS"),
        "Calls" => Some("CALLS"),
        "DeclaredBy" => Some("DECLARED_BY"),
        "Targets" => Some("TARGETS"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::relationship_type;

    #[test]
    fn schema_rejects_unknown_relationship_labels() {
        assert_eq!(relationship_type("Contains"), Some("CONTAINS"));
        assert_eq!(relationship_type("Targets"), Some("TARGETS"));
        assert_eq!(relationship_type("AFFECTS"), None);
    }
}
