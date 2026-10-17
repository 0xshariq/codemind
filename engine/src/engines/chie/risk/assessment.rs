use super::factors::{FactorState, RiskFactors};
use crate::engines::chie::model::{RiskAssessment, RiskLevel};

/// Deterministic qualitative policy. Unknown factors never improve a result:
/// they remain explicit limitations instead of being treated as low risk.
pub fn assess(factors: RiskFactors) -> RiskAssessment {
    let mut rationale = Vec::new();
    let mut unknown = Vec::new();

    if factors.structural.state == FactorState::Supported {
        if factors.structural.maximum_depth >= 3 || factors.structural.impacted_entities >= 4 {
            rationale.push("broad or multi-hop structural exposure is present".to_owned());
        } else if factors.structural.impacted_entities > 0 {
            rationale.push("the selected change has graph-backed dependent exposure".to_owned());
        }
    } else {
        unknown.push("structural exposure is incomplete".to_owned());
    }

    match factors.validation.graph_evidence {
        crate::engines::chie::model::EvidenceAvailability::Verified => {
            rationale.push("impact relationships are resolved graph evidence".to_owned());
        }
        crate::engines::chie::model::EvidenceAvailability::Partial => {
            unknown.push("graph evidence is partial".to_owned());
        }
        crate::engines::chie::model::EvidenceAvailability::Unknown => {
            unknown.push("graph evidence availability is unknown".to_owned());
        }
    }

    if factors.validation.test_candidates == 0 {
        unknown.push("no supported static test mapping was found".to_owned());
    } else {
        rationale.push("supported static test mappings exist".to_owned());
    }

    if !factors.historical.available {
        unknown.push("historical instability data is unavailable".to_owned());
    } else if factors.historical.elevated {
        rationale.push("available historical evidence indicates instability".to_owned());
    }

    if !factors.criticality.known {
        unknown.push("criticality policy is not configured for the selected entity".to_owned());
    } else if factors.criticality.critical {
        rationale.push("the selected entity is marked critical by policy".to_owned());
    }

    let high_risk = factors.criticality.known && factors.criticality.critical
        || factors.structural.maximum_depth >= 3
        || factors.structural.impacted_entities >= 4
        || factors.historical.elevated;
    let exposed = factors.structural.impacted_entities > 0;

    let level = if high_risk {
        RiskLevel::High
    } else if exposed {
        RiskLevel::Medium
    } else if unknown.len() >= 2 {
        RiskLevel::Unknown
    } else {
        RiskLevel::Low
    };

    RiskAssessment {
        level,
        rationale,
        unknown_factors: unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::assess;
    use crate::engines::chie::model::EvidenceAvailability;
    use crate::engines::chie::model::RiskLevel;
    use crate::engines::chie::risk::factors::*;

    #[test]
    fn missing_evidence_never_implies_low_risk_for_exposed_changes() {
        let factors = RiskFactors {
            structural: StructuralExposure {
                impacted_entities: 1,
                maximum_depth: 1,
                state: FactorState::Supported,
            },
            validation: ValidationEvidence {
                test_candidates: 0,
                graph_evidence: EvidenceAvailability::Unknown,
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
                dependency_edges: 1,
            },
        };
        assert_eq!(assess(factors).level, RiskLevel::Medium);
    }
}
