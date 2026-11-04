use super::super::model::{TestEntity, TestNodeKind, TestStatus};
use crate::source::RepositoryContext;

pub fn supported(tests: &[TestEntity]) -> impl Iterator<Item = &TestEntity> {
    tests
        .iter()
        .filter(|test| test.kind == TestNodeKind::Test && test.status == TestStatus::Supported)
}

pub fn supported_for_context<'a>(
    tests: &'a [TestEntity],
    context: &'a RepositoryContext,
    limit: usize,
) -> (Vec<&'a TestEntity>, bool) {
    let mut result = Vec::new();
    let bound = limit.min(1000);
    let mut truncated = false;
    for test in tests.iter().filter(|test| {
        test.kind == TestNodeKind::Test
            && test.status == TestStatus::Supported
            && test.context == *context
    }) {
        if result.len() >= bound {
            truncated = true;
            break;
        }
        result.push(test);
    }
    (result, truncated)
}
