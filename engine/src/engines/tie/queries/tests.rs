use super::super::model::TestEntity;

pub fn supported(tests: &[TestEntity]) -> impl Iterator<Item = &TestEntity> {
    tests
        .iter()
        .filter(|test| test.status == super::super::model::TestStatus::Supported)
}
