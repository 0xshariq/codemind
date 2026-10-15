use super::model::ChangeAnalysis;

pub fn impacted_entity_ids(analysis: &ChangeAnalysis) -> impl Iterator<Item = &str> {
    analysis
        .impact_paths
        .iter()
        .map(|path| path.entity_id.as_str())
}
