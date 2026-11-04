use super::super::model::EvidenceAvailability;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactorState {
    Supported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralExposure {
    pub impacted_entities: usize,
    pub maximum_depth: usize,
    pub state: FactorState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationEvidence {
    pub test_candidates: usize,
    pub graph_evidence: EvidenceAvailability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoricalInstability {
    pub available: bool,
    pub elevated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CriticalityPolicy {
    pub known: bool,
    pub critical: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeCharacteristics {
    pub selection_is_symbol: bool,
    pub dependency_edges: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskFactors {
    pub structural: StructuralExposure,
    pub validation: ValidationEvidence,
    pub historical: HistoricalInstability,
    pub criticality: CriticalityPolicy,
    pub change: ChangeCharacteristics,
}
