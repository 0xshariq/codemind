use super::model::{ChangeAnalysis, Evidence, EvidenceAvailability};
use crate::{engines::coie::model::ResolutionStatus, source::identity::RandomIdGenerator};
use std::collections::HashSet;

pub fn build(analysis: &ChangeAnalysis) -> Vec<Evidence> {
    let mut result = Vec::new();
    let mut id_generator = RandomIdGenerator::default();
    let mut seen_impact_evidence = HashSet::new();

    for path in &analysis.impact_paths {
        for relationship in &path.relationships {
            if relationship.resolution != ResolutionStatus::Resolved
                || !seen_impact_evidence.insert((path.entity_id.as_str(), relationship.id.as_str()))
            {
                continue;
            }
            result.push(Evidence {
                id: id_generator.next("chie-evidence", "impact-path-evidence"),
                relationship_id: Some(relationship.id.clone()),
                entity_id: Some(path.entity_id.clone()),
                test_id: None,
                source: relationship.evidence.clone(),
                derivation: "impact-path relationship".to_owned(),
            });
        }
    }

    for candidate in &analysis.test_candidates {
        result.push(Evidence {
            id: id_generator.next("chie-evidence", "test-target-evidence"),
            relationship_id: None,
            entity_id: candidate.target_entity_id.clone(),
            test_id: Some(candidate.test_id.clone()),
            source: candidate.evidence.clone(),
            derivation: "TIE static target mapping".to_owned(),
        });
    }

    result
}

pub fn availability(analysis: &ChangeAnalysis) -> EvidenceAvailability {
    if let Some(impact) = &analysis.impact {
        return impact.availability;
    }
    if analysis.truncated || !analysis.limitations.is_empty() {
        EvidenceAvailability::Partial
    } else if analysis.evidence_records.is_empty() && analysis.impact_paths.is_empty() {
        EvidenceAvailability::Unknown
    } else {
        EvidenceAvailability::Verified
    }
}

pub fn relationship_ids(analysis: &ChangeAnalysis) -> &[String] {
    &analysis.evidence
}
