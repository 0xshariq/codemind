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
