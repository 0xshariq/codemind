//! Database-independent graph storage facade.
//!
//! A `RepositoryContext.workspace_id` identifies one physical workspace graph in
//! the selected adapter. Code, Test, History, Change, operation, and authorized
//! workspace-scoped memory records are logical domains within that graph.

pub mod falkordb;
pub mod port;

pub use crate::memory::repository::MemoryRepository;
/// Concrete adapter export for the application composition root.
/// Engine and service code should depend on [`GraphStorage`] instead.
pub use falkordb::FalkorDbStorage;
pub use port::{
    ChangeGraphVerificationExpectation, ChangeGraphVerificationReport, DependencyPathResult,
    DerivedRelationship, GraphPath, GraphRetentionReport, GraphStorage,
    GraphVerificationExpectation, GraphVerificationReport, StorageError, StoredTestGraph,
    digest_identity_set, generation_id_for_context,
};

#[cfg(test)]
mod tests {
    use super::DerivedRelationship;
    use crate::source::identity::RandomIdGenerator;

    #[test]
    fn graph_storage_module_exports_the_derived_change_relationship_boundary() {
        let mut id_generator = RandomIdGenerator::default();
        let relationship = DerivedRelationship {
            kind: "AFFECTS".into(),
            from: id_generator.next("chie-impact", "fixture-impact"),
            to: id_generator.next("coie-entity", "fixture-symbol"),
        };
        assert_eq!(relationship.kind, "AFFECTS");
    }
}
