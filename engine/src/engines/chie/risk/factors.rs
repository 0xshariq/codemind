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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::chie::model::RiskLevel;

    #[test]
    fn risk_factor_structures_preserve_unknown_inputs() {
        let factors = RiskFactors {
            structural: StructuralExposure {
                impacted_entities: 2,
                maximum_depth: 2,
                state: FactorState::Supported,
            },
            validation: ValidationEvidence {
                test_candidates: 1,
                graph_evidence: EvidenceAvailability::Verified,
            },
            historical: HistoricalInstability {
                available: false,
                elevated: false,
            },
            criticality: CriticalityPolicy {
                known: false,
                critical: false,
            },
            change: ChangeCharacteristics {
                selection_is_symbol: true,
                dependency_edges: 2,
            },
        };
        let result = crate::engines::chie::risk::assessment::assess(factors);
        assert_eq!(result.level, RiskLevel::Medium);
        assert!(!result.unknown_factors.is_empty());
    }
}
