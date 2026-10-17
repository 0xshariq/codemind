use super::model::{TestEntity, TestRelationship};

/// Canonical in-memory TIE result used by discovery, validation and future
/// pipeline integration. It is deliberately the same pair returned by
/// `TieEngine::discover`, not a second graph representation.
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

#[cfg(test)]
mod tests {
    use crate::engines::tie::discovery::TieEngine;
    use crate::source::SourceProvider;
    use std::fs;

    #[test]
    fn discovery_associates_only_calls_inside_a_test_body() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("lib.rs"),
            "fn helper() {}\nhelper();\n#[test]\nfn works() { helper(); }\n",
        )
        .unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "tie-engine-test")
            .unwrap();
        let (_, relationships) = TieEngine.discover(&snapshot).unwrap();
        let targets: Vec<_> = relationships
            .iter()
            .filter(|relationship| {
                relationship.kind == crate::engines::tie::model::TestRelationshipKind::Targets
            })
            .collect();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].target_symbol_name.as_deref(), Some("helper"));
    }
}
