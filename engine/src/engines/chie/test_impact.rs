use super::model::TestCandidate;
use crate::engines::tie::model::{TestEntity, TestRelationship, TestRelationshipKind};

pub fn candidates(
    tests: &[TestEntity],
    relationships: &[TestRelationship],
    impacted_names: &[String],
) -> Vec<TestCandidate> {
    let mut result = Vec::new();
    for relationship in relationships {
        if relationship.kind != TestRelationshipKind::Targets {
            continue;
        }
        let Some(target) = relationship.target_symbol_name.as_deref() else {
            continue;
        };
        if !impacted_names.iter().any(|name| name == target) {
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
        });
    }
    result
}
