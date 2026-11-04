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
