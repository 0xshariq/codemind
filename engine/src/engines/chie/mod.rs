pub mod change_set;
pub mod engine;
pub mod evidence;
pub mod graph_builder;
pub mod impact;
pub mod model;
pub mod queries;
pub mod risk;
pub mod test_impact;

pub use change_set::{AnalysisRun, ChangeSelectionKind, ChangeSet};
pub use engine::ChieEngine;
pub use model::{
    ChangeAnalysis, ChangeRecord, Evidence, EvidenceAvailability, ImpactAnalysis,
    ImpactClassification, ImpactPath, RiskAssessment, RiskLevel, TestCandidate,
};

#[cfg(test)]
mod integration_tests {
    use super::{ChieEngine, RiskLevel};
    use crate::{
        engines::{
            coie::model::{
                CodeGraph, GraphEntity, GraphRelationship, RelationshipKind, ResolutionStatus,
                SourceEvidence,
            },
            tie::model::{
                TestEntity, TestRelationship, TestRelationshipKind, TestResolution, TestStatus,
            },
        },
        source::RepositoryContext,
    };

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    fn entity(id: &str, name: &str) -> GraphEntity {
        GraphEntity {
            id: id.into(),
            kind: crate::engines::coie::model::EntityKind::Symbol,
            name: name.into(),
            path: "src/lib.rs".into(),
            line: 1,
            context: context(),
            language: None,
            native_kind: Some("function".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: Some(SourceEvidence {
                path: "src/lib.rs".into(),
                start_line: 1,
                end_line: 1,
                snapshot_id: "snapshot".into(),
                provenance: "tree-sitter".into(),
            }),
        }
    }

    #[test]
    fn change_analysis_follows_caller_to_changed_callee_in_reverse() {
        let evidence = SourceEvidence {
            path: "src/lib.rs".into(),
            start_line: 1,
            end_line: 1,
            snapshot_id: "snapshot".into(),
            provenance: "tree-sitter".into(),
        };
        let relationship = GraphRelationship {
            id: "call-1".into(),
            kind: RelationshipKind::Calls,
            from: "caller".into(),
            to: "target".into(),
            context: context(),
            resolution: ResolutionStatus::Resolved,
            evidence: Some(evidence.clone()),
        };
        let graph = CodeGraph {
            context: context(),
            entities: vec![entity("target", "target"), entity("caller", "caller")],
            relationships: vec![relationship],
            diagnostics: Vec::new(),
        };
        let test = TestEntity {
            id: "test-1".into(),
            name: "target regression".into(),
            path: "tests/lib.rs".into(),
            line: 1,
            framework: "rust".into(),
            status: TestStatus::Supported,
            context: context(),
        };
        let mapping = TestRelationship {
            id: "target-1".into(),
            kind: TestRelationshipKind::Targets,
            test_id: test.id.clone(),
            target_symbol_name: Some("target".into()),
            entity_id: Some("target".into()),
            evidence_path: "tests/lib.rs".into(),
            evidence: Some(evidence),
            resolution: TestResolution::Resolved,
            context: context(),
        };

        let analysis = ChieEngine::default()
            .analyze(&graph, &[test], &[mapping], "target")
            .unwrap();

        assert_eq!(analysis.impact_paths[0].entity_id, "caller");
        assert_eq!(
            analysis.impact_paths[0].classification,
            super::ImpactClassification::Direct
        );
        assert!(analysis.test_candidates.is_empty());
        assert!(matches!(
            analysis.risk,
            RiskLevel::Medium | RiskLevel::Unknown
        ));
        assert!(
            analysis
                .evidence_records
                .iter()
                .any(|item| item.relationship_id.as_deref() == Some("call-1"))
        );
    }

    #[test]
    fn missing_selected_entity_is_not_an_empty_success() {
        let graph = CodeGraph {
            context: context(),
            entities: vec![entity("other", "other")],
            relationships: Vec::new(),
            diagnostics: Vec::new(),
        };
        assert!(
            ChieEngine::default()
                .analyze(&graph, &[], &[], "missing")
                .is_err()
        );
    }
}
