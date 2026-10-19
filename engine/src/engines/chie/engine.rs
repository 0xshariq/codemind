use super::{
    change_set::{AnalysisRun, ChangeSelectionKind},
    evidence, graph_builder,
    impact::{aggregation, traversal},
    model::{ChangeAnalysis, ChangeRecord, EvidenceAvailability, ImpactAnalysis},
    risk::{assessment, factors::*},
    test_impact,
};
use chrono::Utc;

use crate::engines::{
    coie::model::{CodeGraph, EntityKind},
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
        graph
            .context
            .validate()
            .map_err(|error| error.to_string())?;

        let selected = graph
            .entities
            .iter()
            .find(|entity| entity.id == selected_entity_id)
            .ok_or_else(|| "selected symbol or file does not exist in the code graph".to_owned())?;

        if selected.context != graph.context {
            return Err("selected entity context does not match graph context".to_owned());
        }

        if !matches!(selected.kind, EntityKind::Symbol | EntityKind::File) {
            return Err("change analysis requires a symbol or file target".to_owned());
        }

        if tests.iter().any(|test| test.context != graph.context)
            || test_relationships
                .iter()
                .any(|relationship| relationship.context != graph.context)
        {
            return Err("test graph context does not match code graph context".to_owned());
        }

        let traversal = traversal::traverse(
            &graph.relationships,
            selected_entity_id,
            self.max_depth,
            crate::contract::result::MAX_PATHS,
        );
        let mut paths = aggregation::deduplicate(traversal.paths);
        let truncated = traversal.truncated || paths.len() > crate::contract::result::MAX_PATHS;
        paths.truncate(crate::contract::result::MAX_PATHS);

        let impacted_ids: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            paths
                .iter()
                .filter_map(|path| {
                    seen.insert(path.entity_id.clone())
                        .then_some(path.entity_id.clone())
                })
                .collect()
        };
        let candidates = test_impact::candidates(tests, test_relationships, &impacted_ids);

        let maximum_depth = paths.iter().map(|path| path.depth).max().unwrap_or(0);
        let graph_evidence = if paths.iter().all(|path| {
            path.relationships.iter().all(|relationship| {
                relationship.evidence.is_some()
                    && matches!(
                        relationship.resolution,
                        crate::engines::coie::model::ResolutionStatus::Resolved
                    )
            })
        }) && !paths.is_empty()
        {
            EvidenceAvailability::Verified
        } else if !paths.is_empty() {
            EvidenceAvailability::Partial
        } else {
            EvidenceAvailability::Unknown
        };

        let risk_assessment = assessment::assess(RiskFactors {
            structural: StructuralExposure {
                impacted_entities: impacted_ids.len(),
                maximum_depth,
                state: FactorState::Supported,
            },
            validation: ValidationEvidence {
                test_candidates: candidates.len(),
                graph_evidence,
            },
            historical: HistoricalInstability {
                available: false,
                elevated: false,
            },
            criticality: CriticalityPolicy {
                known: false,
                critical: false,
            },
            change: ChangeCharacteristics {
                selection_is_symbol: matches!(selected.kind, EntityKind::Symbol),
                dependency_edges: paths.len(),
            },
        });

        let mut analysis = ChangeAnalysis {
            analysis_id: format!(
                "analysis-{}",
                crate::source::identity::digest_text(&format!(
                    "{}:{}:{}:{}",
                    graph.context.repository_id,
                    graph.context.source_snapshot_id,
                    selected_entity_id,
                    Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ))
            ),
            context: graph.context.clone(),
            selected_entity_id: selected_entity_id.to_owned(),
            impact_paths: paths.clone(),
            test_candidates: candidates,
            risk: risk_assessment.level,
            risk_assessment: Some(risk_assessment),
            impact: Some(ImpactAnalysis {
                impacted_entity_ids: impacted_ids,
                paths,
                evidence: Vec::new(),
                availability: graph_evidence,
                truncated,
            }),
            evidence: Vec::new(),
            evidence_records: Vec::new(),
            limitations: vec![
                "Impact is derived from bounded static dependency relationships.".to_owned(),
                "Static test declarations are not runtime coverage evidence.".to_owned(),
                "Historical instability and criticality policy were not available in this analysis.".to_owned(),
            ],
            truncated,
        };

        if truncated {
            analysis
                .limitations
                .push("Impact paths were truncated at the maximum path limit.".to_owned());
        }

        analysis.evidence_records = evidence::build(&analysis);
        analysis.evidence = analysis
            .evidence_records
            .iter()
            .filter_map(|record| record.relationship_id.clone())
            .collect();

        if let Some(impact) = analysis.impact.as_mut() {
            impact.evidence = analysis
                .evidence_records
                .iter()
                .filter_map(|record| record.source.clone())
                .collect();
        }

        graph_builder::validate(&analysis)?;
        Ok(analysis)
    }

    pub fn change_record(
        &self,
        change_set_id: &str,
        graph: &CodeGraph,
        selected_entity_id: &str,
    ) -> Result<ChangeRecord, String> {
        let entity = graph
            .entities
            .iter()
            .find(|entity| entity.id == selected_entity_id)
            .ok_or_else(|| "selected entity not found".to_owned())?;

        let selection_kind = match entity.kind {
            EntityKind::Symbol => ChangeSelectionKind::Symbol,
            EntityKind::File => ChangeSelectionKind::File,
            _ => return Err("selected entity is not a supported change target".to_owned()),
        };

        Ok(ChangeRecord {
            change_set_id: change_set_id.to_owned(),
            selected_entity_id: selected_entity_id.to_owned(),
            context: graph.context.clone(),
            selection_kind: match selection_kind {
                ChangeSelectionKind::Symbol => "symbol",
                ChangeSelectionKind::File => "file",
            }
            .to_owned(),
            source: entity.evidence.clone(),
        })
    }

    pub fn analysis_run(
        &self,
        id: &str,
        change_set_id: &str,
        context: crate::source::RepositoryContext,
        started_at: &str,
    ) -> Result<AnalysisRun, String> {
        let run = AnalysisRun {
            id: id.to_owned(),
            change_set_id: change_set_id.to_owned(),
            context,
            status: "completed".to_owned(),
            started_at: started_at.to_owned(),
            completed_at: Some(started_at.to_owned()),
        };
        run.validate()?;
        Ok(run)
    }
}
