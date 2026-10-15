use crate::{
    engines::coie::model::{GraphRelationship, RelationshipKind},
    source::RepositoryContext,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpactClassification {
    Direct,
    Indirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Unknown,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImpactPath {
    pub entity_id: String,
    pub classification: ImpactClassification,
    pub depth: usize,
    pub relationships: Vec<GraphRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestCandidate {
    pub test_id: String,
    pub name: String,
    pub path: String,
    pub evidence_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeAnalysis {
    pub context: RepositoryContext,
    pub selected_entity_id: String,
    pub impact_paths: Vec<ImpactPath>,
    pub test_candidates: Vec<TestCandidate>,
    pub risk: RiskLevel,
    pub evidence: Vec<String>,
    pub limitations: Vec<String>,
    pub truncated: bool,
}

pub fn traversable(kind: RelationshipKind) -> bool {
    matches!(kind, RelationshipKind::Calls | RelationshipKind::Imports)
}
