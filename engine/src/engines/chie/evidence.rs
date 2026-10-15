use super::model::ChangeAnalysis;

pub fn relationship_ids(analysis: &ChangeAnalysis) -> &[String] {
    &analysis.evidence
}
