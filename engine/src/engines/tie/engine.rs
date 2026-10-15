use super::model::{TestEntity, TestRelationship};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestGraph {
    pub tests: Vec<TestEntity>,
    pub relationships: Vec<TestRelationship>,
}

pub fn graph(tests: Vec<TestEntity>, relationships: Vec<TestRelationship>) -> TestGraph {
    TestGraph {
        tests,
        relationships,
    }
}
