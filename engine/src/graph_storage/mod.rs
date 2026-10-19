pub mod falkordb;
pub mod port;

pub use crate::memory::repository::MemoryRepository;
pub use falkordb::FalkorDbStorage;
pub use port::{
    DependencyPathResult, DerivedRelationship, GraphPath, GraphStorage, StorageError,
    StoredTestGraph,
};

#[cfg(test)]
mod tests {
    use super::DerivedRelationship;

    #[test]
    fn graph_storage_module_exports_the_derived_change_relationship_boundary() {
        let relationship = DerivedRelationship {
            kind: "AFFECTS".into(),
            from: "impact".into(),
            to: "symbol".into(),
        };
        assert_eq!(relationship.kind, "AFFECTS");
    }
}
