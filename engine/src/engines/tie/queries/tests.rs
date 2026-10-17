use super::super::model::{TestEntity, TestStatus};
use crate::source::RepositoryContext;

pub fn supported(tests: &[TestEntity]) -> impl Iterator<Item = &TestEntity> {
    tests
        .iter()
        .filter(|test| test.status == TestStatus::Supported)
}

pub fn supported_for_context<'a>(
    tests: &'a [TestEntity],
    context: &'a RepositoryContext,
    limit: usize,
) -> (Vec<&'a TestEntity>, bool) {
    let mut result = Vec::new();
    let bound = limit.min(1000);
    let mut truncated = false;
    for test in tests
        .iter()
        .filter(|test| test.status == TestStatus::Supported && test.context == *context)
    {
        if result.len() >= bound {
            truncated = true;
            break;
        }
        result.push(test);
    }
    (result, truncated)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::source::RepositoryContext;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn supported_query_filters_status_and_context_with_a_bound() {
        let context = context();
        let supported_test = TestEntity {
            id: "supported".into(),
            name: "supported".into(),
            path: "tests/lib.rs".into(),
            line: 1,
            framework: "rust".into(),
            status: TestStatus::Supported,
            context: context.clone(),
        };
        let partial_test = TestEntity {
            status: TestStatus::Partial,
            id: "partial".into(),
            ..supported_test.clone()
        };
        let tests = vec![supported_test.clone(), partial_test];
        let (result, truncated) = supported_for_context(&tests, &context, 1);
        assert_eq!(result.len(), 1);
        assert!(!truncated);
        assert_eq!(supported(&[supported_test]).count(), 1);
    }
}
