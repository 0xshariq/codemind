use super::model::ChangeAnalysis;

pub fn validate(analysis: &ChangeAnalysis) -> bool {
    analysis
        .impact_paths
        .iter()
        .all(|path| !path.relationships.is_empty() && path.depth == path.relationships.len())
}
