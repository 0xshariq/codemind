use super::model::{ChangeAnalysis, Evidence, EvidenceAvailability};
use crate::engines::coie::model::ResolutionStatus;

pub fn build(analysis: &ChangeAnalysis) -> Vec<Evidence> {
    let mut result = Vec::new();

    for path in &analysis.impact_paths {
        for relationship in &path.relationships {
            if relationship.resolution != ResolutionStatus::Resolved {
                continue;
            }
            result.push(Evidence {
                id: format!(
                    "evidence:{}:{}:{}:{}",
                    analysis.analysis_id,
                    analysis.selected_entity_id,
                    path.entity_id,
                    relationship.id
                ),
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
            id: format!(
                "test-evidence:{}:{}:{}:{}",
                analysis.analysis_id,
                analysis.selected_entity_id,
                candidate
                    .target_entity_id
                    .as_deref()
                    .unwrap_or("<unresolved>"),
                candidate.test_id
            ),
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
