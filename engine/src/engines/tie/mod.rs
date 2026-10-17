pub mod discovery;
pub mod engine;
pub mod graph_builder;
pub mod mapping;
pub mod model;
pub mod queries;

pub use discovery::TieEngine;
pub use engine::TestGraph;
pub use model::{TestEntity, TestRelationship, TestRelationshipKind, TestResolution, TestStatus};

#[cfg(test)]
mod tests {
    use super::{TestRelationshipKind, TestResolution, TestStatus, TieEngine};
    use crate::source::SourceProvider;
    use std::fs;

    #[test]
    fn level_two_discovery_binds_targets_to_canonical_coie_ids() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("lib.rs"),
            "fn helper() {}\n\n#[test]\nfn test_helper() { helper(); }\n",
        )
        .unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "tie-test")
            .unwrap();
        let coie = crate::engines::coie::CoieEngine.extract(&snapshot).unwrap();
        let (tests, relationships) = TieEngine.discover(&snapshot).unwrap();
        let (tests, relationships) = crate::engines::tie::mapping::target_mapping::resolve_graph(
            &coie,
            tests,
            relationships,
        );

        assert_eq!(tests.len(), 1);
        assert!(relationships.iter().any(|relationship| {
            relationship.kind == TestRelationshipKind::DeclaredBy
                && relationship.resolution == TestResolution::Resolved
                && relationship.entity_id.is_some()
        }));
        let target = relationships
            .iter()
            .find(|relationship| relationship.kind == TestRelationshipKind::Targets)
            .expect("test body call should produce a target mapping");
        assert_eq!(target.target_symbol_name.as_deref(), Some("helper"));
        assert_eq!(target.resolution, TestResolution::Resolved);
        assert!(
            target
                .entity_id
                .as_deref()
                .is_some_and(|id| id.starts_with("symbol:"))
        );
        assert_eq!(tests[0].status, TestStatus::Supported);
        assert!(target.evidence.is_some());
    }

    #[test]
    fn level_two_does_not_match_test_markers_in_comments_or_strings() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("lib.rs"),
            "// #[test]\nfn helper() {}\nconst TEXT: &str = \"#[test]\";\n",
        )
        .unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "tie-test")
            .unwrap();
        let (tests, _) = TieEngine.discover(&snapshot).unwrap();
        assert!(tests.is_empty());
    }
}

#[cfg(test)]
mod level_two_language_tests {
    use super::{TestRelationshipKind, TieEngine};
    use crate::source::SourceProvider;
    use std::fs;

    #[test]
    fn level_two_maps_javascript_test_body_only_to_calls_inside_that_callback() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("test.js"),
            "function helper() {}\nhelper();\ntest('works', () => helper());\n",
        )
        .unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "tie-js-test")
            .unwrap();
        let (_, relationships) = TieEngine.discover(&snapshot).unwrap();
        let targets: Vec<_> = relationships
            .iter()
            .filter(|r| r.kind == TestRelationshipKind::Targets)
            .collect();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].target_symbol_name.as_deref(), Some("helper"));
    }
}
