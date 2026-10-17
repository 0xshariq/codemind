use crate::{
    engines::coie::model::{GraphRelationship, RelationshipKind, SourceEvidence},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceAvailability {
    Verified,
    Partial,
    Unknown,
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
    pub target_entity_id: Option<String>,
    pub evidence: Option<SourceEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeRecord {
    pub change_set_id: String,
    pub selected_entity_id: String,
    pub context: RepositoryContext,
    pub selection_kind: String,
    pub source: Option<SourceEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImpactAnalysis {
    pub impacted_entity_ids: Vec<String>,
    pub paths: Vec<ImpactPath>,
    pub evidence: Vec<SourceEvidence>,
    pub availability: EvidenceAvailability,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskAssessment {
    pub level: RiskLevel,
    pub rationale: Vec<String>,
    pub unknown_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub id: String,
    pub relationship_id: Option<String>,
    pub entity_id: Option<String>,
    pub test_id: Option<String>,
    pub source: Option<SourceEvidence>,
    pub derivation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeAnalysis {
    pub context: RepositoryContext,
    pub selected_entity_id: String,
    pub impact_paths: Vec<ImpactPath>,
    pub test_candidates: Vec<TestCandidate>,
    pub risk: RiskLevel,
    pub risk_assessment: Option<RiskAssessment>,
    pub impact: Option<ImpactAnalysis>,
    pub evidence: Vec<String>,
    pub evidence_records: Vec<Evidence>,
    pub limitations: Vec<String>,
    pub truncated: bool,
}

pub fn traversable(kind: &RelationshipKind) -> bool {
    matches!(kind, RelationshipKind::Calls | RelationshipKind::Imports)
}
