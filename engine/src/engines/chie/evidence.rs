use super::model::{ChangeAnalysis, Evidence, EvidenceAvailability};
use crate::engines::coie::model::ResolutionStatus;

pub fn build(analysis: &ChangeAnalysis) -> Vec<Evidence> {
    let mut result = Vec::new();

    for path in &analysis.impact_paths {
        for relationship in &path.relationships {
            if relationship.resolution != ResolutionStatus::Resolved {
                continue;
            }
            result.push(Evidence {
                id: format!("evidence:{}", relationship.id),
                relationship_id: Some(relationship.id.clone()),
                entity_id: Some(path.entity_id.clone()),
                test_id: None,
                source: relationship.evidence.clone(),
                derivation: "impact-path relationship".to_owned(),
            });
        }
    }

    for candidate in &analysis.test_candidates {
        result.push(Evidence {
            id: format!("test-evidence:{}", candidate.test_id),
            relationship_id: None,
            entity_id: candidate.target_entity_id.clone(),
            test_id: Some(candidate.test_id.clone()),
            source: candidate.evidence.clone(),
            derivation: "TIE static target mapping".to_owned(),
        });
    }

    result
}

pub fn availability(analysis: &ChangeAnalysis) -> EvidenceAvailability {
    if let Some(impact) = &analysis.impact {
        return impact.availability;
    }
    if analysis.truncated || !analysis.limitations.is_empty() {
        EvidenceAvailability::Partial
    } else if analysis.evidence_records.is_empty() && analysis.impact_paths.is_empty() {
        EvidenceAvailability::Unknown
    } else {
        EvidenceAvailability::Verified
    }
}

pub fn relationship_ids(analysis: &ChangeAnalysis) -> &[String] {
    &analysis.evidence
}

#[cfg(test)]
mod tests {
    use super::build;
    use crate::engines::chie::model::{
        ChangeAnalysis, EvidenceAvailability, ImpactClassification, ImpactPath, RiskLevel,
        TestCandidate,
    };
    use crate::engines::coie::model::{
        GraphRelationship, RelationshipKind, ResolutionStatus, SourceEvidence,
    };
    use crate::source::RepositoryContext;

    #[test]
    fn evidence_builder_keeps_only_resolved_impact_and_test_evidence() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let resolved = GraphRelationship {
            id: "call".into(),
            kind: RelationshipKind::Calls,
            from: "a".into(),
            to: "b".into(),
            context: context.clone(),
            resolution: ResolutionStatus::Resolved,
            evidence: Some(SourceEvidence {
                path: "src/lib.rs".into(),
                start_line: 1,
                end_line: 2,
                snapshot_id: "snapshot".into(),
                provenance: "test".into(),
            }),
        };
        let unresolved = GraphRelationship {
            resolution: ResolutionStatus::Unresolved,
            id: "unresolved".into(),
            ..resolved.clone()
        };
        let analysis = ChangeAnalysis {
            selected_entity_id: "b".into(),
            context,
            impact_paths: vec![ImpactPath {
                entity_id: "a".into(),
                depth: 1,
                classification: ImpactClassification::Direct,
                relationships: vec![resolved, unresolved],
            }],
            impact: None,
            test_candidates: vec![TestCandidate {
                test_id: "test-1".into(),
                name: "test".into(),
                path: "tests/lib.rs".into(),
                evidence_path: "tests/lib.rs".into(),
                target_entity_id: Some("b".into()),
                evidence: None,
            }],
            risk: RiskLevel::Unknown,
            risk_assessment: None,
            evidence: Vec::new(),
            evidence_records: Vec::new(),
            limitations: vec!["test limitation".into()],
            truncated: false,
        };
        assert_eq!(
            super::availability(&analysis),
            EvidenceAvailability::Partial
        );
        let evidence = build(&analysis);
        assert_eq!(evidence.len(), 2);
        assert!(evidence.iter().any(|item| item.id == "evidence:call"));
        assert!(
            evidence
                .iter()
                .any(|item| item.id == "test-evidence:test-1")
        );
    }
}
