use crate::engines::coie::model::SourceEvidence;
use crate::source::RepositoryContext;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Supported,
    Partial,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestRelationshipKind {
    DeclaredBy,
    Targets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestResolution {
    Resolved,
    Unresolved,
    Ambiguous,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestEntity {
    pub id: String,
    pub name: String,
    pub path: String,
    pub line: u32,
    pub framework: String,
    pub status: TestStatus,
    pub context: RepositoryContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRelationship {
    pub id: String,
    pub kind: TestRelationshipKind,
    pub test_id: String,
    pub target_symbol_name: Option<String>,
    pub entity_id: Option<String>,
    pub evidence_path: String,
    pub evidence: Option<SourceEvidence>,
    pub resolution: TestResolution,
    pub context: RepositoryContext,
}
