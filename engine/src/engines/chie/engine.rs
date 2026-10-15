use super::{
    impact::{aggregation, traversal},
    model::ChangeAnalysis,
    risk::{assessment, factors::RiskFactors},
    test_impact,
};
use crate::engines::{
    coie::model::CodeGraph,
    tie::model::{TestEntity, TestRelationship},
};

#[derive(Debug, Clone, Copy)]
pub struct ChieEngine {
    pub max_depth: usize,
}

impl Default for ChieEngine {
    fn default() -> Self {
        Self { max_depth: 4 }
    }
}

impl ChieEngine {
    pub fn analyze(
        &self,
        graph: &CodeGraph,
        tests: &[TestEntity],
        test_relationships: &[TestRelationship],
        selected_entity_id: &str,
    ) -> Result<ChangeAnalysis, String> {
        if graph.context
            != tests
                .first()
                .map(|test| &test.context)
                .cloned()
                .unwrap_or_else(|| graph.context.clone())
        {
            return Err("test graph context does not match code graph context".to_owned());
        }
        let paths = aggregation::deduplicate(traversal::traverse(
            &graph.relationships,
            selected_entity_id,
            self.max_depth,
        ));
        let impacted_names: Vec<_> = paths
            .iter()
            .filter_map(|path| {
                graph
                    .entities
                    .iter()
                    .find(|entity| entity.id == path.entity_id)
            })
            .map(|entity| entity.name.clone())
            .collect();
        let candidates = test_impact::candidates(tests, test_relationships, &impacted_names);
        let maximum_depth = paths.iter().map(|path| path.depth).max().unwrap_or(0);
        let risk = assessment::assess(RiskFactors {
            impacted_entities: paths.len(),
            maximum_depth,
            has_test_evidence: !candidates.is_empty(),
            has_graph_evidence: !paths.is_empty(),
        });
        let evidence = paths
            .iter()
            .flat_map(|path| {
                path.relationships
                    .iter()
                    .map(|relationship| relationship.id.clone())
            })
            .collect();
        Ok(ChangeAnalysis {
            context: graph.context.clone(),
            selected_entity_id: selected_entity_id.to_owned(),
            impact_paths: paths,
            test_candidates: candidates,
            risk,
            evidence,
            limitations: vec![
                "Analysis uses bounded static CALLS and IMPORTS relationships.".to_owned(),
                "Static test declarations are not runtime coverage evidence.".to_owned(),
            ],
        })
    }
}
