pub const CODE_ENTITY_LABEL: &str = "CodeEntity";
pub const TEST_ENTITY_LABEL: &str = "TestEntity";

pub fn relationship_type(kind: &str) -> Option<&'static str> {
    match kind {
        "Defines" => Some("DEFINES"),
        "Imports" => Some("IMPORTS"),
        "Calls" => Some("CALLS"),
        "DeclaredBy" => Some("DECLARED_BY"),
        "Targets" => Some("TARGETS"),
        _ => None,
    }
}
