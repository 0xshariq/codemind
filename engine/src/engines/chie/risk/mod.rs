pub mod assessment;
pub mod factors;

#[cfg(test)]
mod integration_tests {
    use super::{assessment::assess, factors::*};
    use crate::engines::chie::model::{EvidenceAvailability, RiskLevel};

    #[test]
    fn risk_module_combines_structural_and_validation_factors() {
        let factors = RiskFactors {
            structural: StructuralExposure {
                impacted_entities: 5,
                maximum_depth: 3,
                state: FactorState::Supported,
            },
            validation: ValidationEvidence {
                test_candidates: 2,
                graph_evidence: EvidenceAvailability::Verified,
            },
            historical: HistoricalInstability {
                available: true,
                elevated: false,
            },
            criticality: CriticalityPolicy {
                known: true,
                critical: false,
            },
            change: ChangeCharacteristics {
                selection_is_symbol: true,
                dependency_edges: 5,
            },
        };
        assert_eq!(assess(factors).level, RiskLevel::High);
    }
}
