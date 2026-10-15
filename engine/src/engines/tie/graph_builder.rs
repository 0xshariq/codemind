use super::model::{TestEntity, TestRelationship};

pub fn validate(tests: &[TestEntity], relationships: &[TestRelationship]) -> bool {
    relationships.iter().all(|relationship| {
        tests
            .iter()
            .any(|test| test.id == relationship.test_id && test.context == relationship.context)
    })
}
