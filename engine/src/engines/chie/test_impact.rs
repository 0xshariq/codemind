use super::model::TestCandidate;
use crate::engines::tie::model::{
    TestEntity, TestRelationship, TestRelationshipKind, TestResolution,
};

pub fn candidates(
    tests: &[TestEntity],
    relationships: &[TestRelationship],
    impacted_ids: &[String],
) -> Vec<TestCandidate> {
    let mut result = Vec::new();

    for relationship in relationships {
        if relationship.kind != TestRelationshipKind::Targets
            || relationship.resolution != TestResolution::Resolved
        {
            continue;
        }
        let Some(target_id) = relationship.entity_id.as_deref() else {
            continue;
        };
        if !impacted_ids.iter().any(|id| id == target_id) {
            continue;
        }
        let Some(test) = tests.iter().find(|test| test.id == relationship.test_id) else {
            continue;
        };
        if result
            .iter()
            .any(|candidate: &TestCandidate| candidate.test_id == test.id)
        {
            continue;
        }

        result.push(TestCandidate {
            test_id: test.id.clone(),
            name: test.name.clone(),
            path: test.path.clone(),
            evidence_path: relationship.evidence_path.clone(),
            target_entity_id: relationship.entity_id.clone(),
            evidence: relationship.evidence.clone(),
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::candidates;
    use crate::engines::tie::model::*;
    use crate::source::RepositoryContext;

    #[test]
    fn test_candidates_are_deduplicated_by_test_id() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let test = TestEntity {
            id: "test".into(),
            name: "test".into(),
            path: "tests/lib.rs".into(),
            line: 1,
            framework: "rust".into(),
            status: TestStatus::Supported,
            context: context.clone(),
        };
        let relationship = |id: &str| TestRelationship {
            id: id.into(),
            kind: TestRelationshipKind::Targets,
            test_id: "test".into(),
            target_symbol_name: Some("target".into()),
            entity_id: Some("target".into()),
            evidence_path: "tests/lib.rs".into(),
            evidence: None,
            resolution: TestResolution::Resolved,
            context: context.clone(),
        };
        let result = candidates(
            &[test],
            &[relationship("a"), relationship("b")],
            &["target".into()],
        );
        assert_eq!(result.len(), 1);
    }
}
