#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskFactors {
    pub impacted_entities: usize,
    pub maximum_depth: usize,
    pub has_test_evidence: bool,
    pub has_graph_evidence: bool,
}
