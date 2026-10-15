use super::factors::RiskFactors;
use crate::engines::chie::model::RiskLevel;

pub fn assess(factors: RiskFactors) -> RiskLevel {
    if !factors.has_graph_evidence {
        return RiskLevel::Unknown;
    }
    if factors.impacted_entities >= 4 || factors.maximum_depth >= 3 {
        return RiskLevel::High;
    }
    if !factors.has_test_evidence || factors.impacted_entities >= 2 {
        return RiskLevel::Medium;
    }
    RiskLevel::Low
}
