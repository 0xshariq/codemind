use super::model::{ChangeAnalysis, EvidenceAvailability};
use crate::engines::coie::model::ResolutionStatus;
use std::collections::HashSet;

pub fn validate(analysis: &ChangeAnalysis) -> Result<(), String> {
    analysis
        .context
        .validate()
        .map_err(|error| error.to_string())?;

    if analysis.selected_entity_id.trim().is_empty() {
        return Err("selected entity is required".to_owned());
    }

    let mut relationship_ids = HashSet::new();
    for path in &analysis.impact_paths {
        if path.entity_id.trim().is_empty()
            || path.relationships.is_empty()
            || path.depth != path.relationships.len()
        {
            return Err("impact path is structurally invalid".to_owned());
        }
        for relationship in &path.relationships {
            if relationship.context != analysis.context {
                return Err("impact evidence context mismatch".to_owned());
            }
            if relationship.resolution != ResolutionStatus::Resolved {
                return Err("unresolved relationship cannot support Change Graph impact".to_owned());
            }
            if !relationship_ids.insert(relationship.id.clone()) {
                continue;
            }
            if relationship.evidence.is_none() {
                return Err("impact relationship is missing source evidence".to_owned());
            }
        }
    }

    if analysis.impact.as_ref().map(|impact| impact.availability)
        == Some(EvidenceAvailability::Verified)
        && analysis
            .impact
            .as_ref()
            .map(|impact| impact.paths.is_empty())
            .unwrap_or(false)
    {
        return Err("verified impact cannot be empty".to_owned());
    }

    for evidence in &analysis.evidence_records {
        if evidence.id.trim().is_empty() {
            return Err("derived evidence requires an ID".to_owned());
        }
        if let Some(source) = &evidence.source
            && source.snapshot_id != analysis.context.source_snapshot_id
        {
            return Err("derived evidence snapshot mismatch".to_owned());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;
    use crate::engines::chie::model::ChangeAnalysis;
    use crate::source::RepositoryContext;

    #[test]
    fn graph_builder_rejects_missing_selection() {
        let analysis = ChangeAnalysis {
            analysis_id: "analysis-test".into(),
            context: RepositoryContext {
                repository_id: "repo".into(),
                workspace_id: "workspace".into(),
                source_snapshot_id: "snapshot".into(),
                graph_version: "graph".into(),
            },
            selected_entity_id: String::new(),
            impact_paths: Vec::new(),
            test_candidates: Vec::new(),
            risk: crate::engines::chie::model::RiskLevel::Unknown,
            risk_assessment: None,
            impact: None,
            evidence: Vec::new(),
            evidence_records: Vec::new(),
            limitations: Vec::new(),
            truncated: false,
        };
        assert!(validate(&analysis).is_err());
    }
}
