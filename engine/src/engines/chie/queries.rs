use super::model::{ChangeAnalysis, ImpactClassification};
use crate::contract::result::{CanonicalResult, Completeness, Correlation, Freshness, Outcome};

pub fn impacted_entity_ids(analysis: &ChangeAnalysis) -> impl Iterator<Item = &str> {
    analysis
        .impact_paths
        .iter()
        .map(|path| path.entity_id.as_str())
}

pub fn bounded_impacted_entity_ids(
    analysis: &ChangeAnalysis,
    limit: usize,
) -> Result<(Vec<String>, bool), String> {
    if limit == 0 {
        return Err("query limit must be greater than zero".to_owned());
    }
    let mut ids = Vec::new();
    for id in impacted_entity_ids(analysis) {
        if ids.iter().any(|existing| existing == id) {
            continue;
        }
        if ids.len() == limit {
            return Ok((ids, true));
        }
        ids.push(id.to_owned());
    }
    Ok((ids, false))
}

pub fn canonical_summary(
    analysis: &ChangeAnalysis,
    correlation: Correlation,
) -> Result<CanonicalResult<Vec<String>>, String> {
    let (ids, truncated) =
        bounded_impacted_entity_ids(analysis, crate::contract::result::MAX_RELATIONSHIPS)?;
    let completeness = if truncated || analysis.truncated {
        Completeness::Truncated
    } else {
        Completeness::Complete
    };
    let outcome = if truncated || analysis.truncated {
        Outcome::Partial
    } else if ids.is_empty() {
        Outcome::Empty
    } else {
        Outcome::Success
    };
    let result = CanonicalResult::new(
        analysis.context.clone(),
        correlation,
        outcome,
        completeness,
        Freshness::Current,
        ids,
    );
    result.validate().map_err(|error| error.to_string())?;
    Ok(result)
}

pub fn classification(analysis: &ChangeAnalysis) -> (usize, usize) {
    analysis
        .impact_paths
        .iter()
        .fold((0, 0), |(direct, indirect), path| {
            match path.classification {
                ImpactClassification::Direct => (direct + 1, indirect),
                ImpactClassification::Indirect => (direct, indirect + 1),
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::chie::model::{ChangeAnalysis, ImpactClassification, RiskLevel};
    use crate::source::RepositoryContext;

    fn analysis() -> ChangeAnalysis {
        ChangeAnalysis {
            context: RepositoryContext {
                repository_id: "repo".into(),
                workspace_id: "workspace".into(),
                source_snapshot_id: "snapshot".into(),
                graph_version: "graph".into(),
            },
            selected_entity_id: "selected".into(),
            impact_paths: vec![crate::engines::chie::model::ImpactPath {
                entity_id: "a".into(),
                depth: 1,
                classification: ImpactClassification::Direct,
                relationships: Vec::new(),
            }],
            test_candidates: Vec::new(),
            risk: RiskLevel::Unknown,
            risk_assessment: None,
            impact: None,
            evidence: Vec::new(),
            evidence_records: Vec::new(),
            limitations: Vec::new(),
            truncated: false,
        }
    }

    #[test]
    fn bounded_queries_deduplicate_and_report_truncation() {
        let analysis = analysis();
        assert_eq!(
            bounded_impacted_entity_ids(&analysis, 1).unwrap(),
            (vec!["a".to_owned()], false)
        );
        assert!(bounded_impacted_entity_ids(&analysis, 0).is_err());
        let result = canonical_summary(
            &analysis,
            Correlation {
                request_id: "r".into(),
                trace_id: "t".into(),
                operation_id: "o".into(),
            },
        )
        .unwrap();
        assert_eq!(result.outcome, Outcome::Success);
        assert_eq!(classification(&analysis), (1, 0));
    }
}
