use crate::engines::coie::model::SourceEvidence;
use crate::source::RepositoryContext;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Supported,
    Partial,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TestNodeKind {
    #[default]
    Test,
    TestSuite,
    Fixture,
    TestCase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestRelationshipKind {
    Contains,
    UsesFixture,
    DependsOn,
    DeclaredBy,
    Targets,
    HasCase,
    SetupBy,
    TeardownBy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TestTargetKind {
    #[default]
    Code,
    Test,
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
    #[serde(default)]
    pub kind: TestNodeKind,
    pub context: RepositoryContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRelationship {
    pub id: String,
    pub kind: TestRelationshipKind,
    /// ID of the source Test Graph node. Kept as `test_id` for compatibility
    /// with existing ChIE/test-impact consumers.
    pub test_id: String,
    pub target_symbol_name: Option<String>,
    /// Target ID. For Code targets this is a canonical CoIE entity; for Test
    /// targets this is another Test Graph node.
    pub entity_id: Option<String>,
    #[serde(default)]
    pub target_kind: TestTargetKind,
    pub evidence_path: String,
    pub evidence: Option<SourceEvidence>,
    pub resolution: TestResolution,
    pub context: RepositoryContext,
}
