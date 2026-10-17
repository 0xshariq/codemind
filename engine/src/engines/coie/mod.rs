pub mod engine;
pub mod extraction;
pub mod graph_builder;
pub mod languages;
pub mod model;
pub mod queries;

pub use engine::CoieEngine;
pub use model::{
    CodeGraph, Diagnostic, EntityKind, GraphBuildReport, GraphEntity, GraphRelationship,
    RelationshipKind, ResolutionStatus, SourceEvidence,
};

#[cfg(test)]
mod integration_tests {
    use super::{CoieEngine, EntityKind, RelationshipKind, graph_builder};
    use crate::source::SourceProvider;
    use std::fs;

    #[test]
    fn coie_extracts_scoped_symbols_and_evidence_from_one_snapshot() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(
            root.path().join("src/lib.rs"),
            "fn target() {}\nfn caller() { target(); }\n",
        )
        .unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "coie-test")
            .unwrap();
        let graph = CoieEngine.extract(&snapshot).unwrap();
        assert!(graph_builder::validate(&graph));
        let caller = graph
            .entities
            .iter()
            .find(|e| e.name == "caller" && e.kind == EntityKind::Symbol)
            .unwrap();
        let target = graph
            .entities
            .iter()
            .find(|e| e.name == "target" && e.kind == EntityKind::Symbol)
            .unwrap();
        assert!(
            graph
                .relationships
                .iter()
                .any(|r| r.kind == RelationshipKind::Calls
                    && r.from == caller.id
                    && r.to == target.id)
        );
        assert!(caller.evidence.is_some());
        assert!(target.evidence.is_some());
    }

    #[test]
    fn publishes_repository_and_snapshot_provenance_nodes() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/lib.rs"), "fn target() {}\n").unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "coie-provenance")
            .unwrap();
        let graph = CoieEngine.extract(&snapshot).unwrap();
        let repository = graph
            .entities
            .iter()
            .find(|entity| entity.kind == EntityKind::Repository)
            .unwrap();
        let source_snapshot = graph
            .entities
            .iter()
            .find(|entity| entity.kind == EntityKind::RepositorySnapshot)
            .unwrap();
        assert!(graph.relationships.iter().any(|relationship| {
            relationship.kind == RelationshipKind::Contains
                && relationship.from == repository.id
                && relationship.to == source_snapshot.id
        }));
        assert!(graph_builder::validate(&graph));
    }

    #[test]
    fn duplicate_names_do_not_create_ambiguous_call_edges() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("lib.rs"), "struct A; struct B; impl A { fn target() {} } impl B { fn target() {} } fn caller() { target(); }\n").unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "coie-ambiguous")
            .unwrap();
        let graph = CoieEngine.extract(&snapshot).unwrap();
        assert!(
            !graph
                .relationships
                .iter()
                .any(|r| r.kind == RelationshipKind::Calls)
        );
        assert!(graph.diagnostics.iter().any(|d| d.code == "CALL_AMBIGUOUS"));
    }
}
