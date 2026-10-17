pub mod client;
pub mod mapping;
pub mod read;
pub mod schema;
pub mod write;

pub use client::FalkorDbStorage;

#[cfg(test)]
mod integration_tests {
    use super::{mapping::code_relationship_label, read::bounded_limit, write::stable_merge_key};
    use crate::engines::coie::model::RelationshipKind;

    #[test]
    fn storage_mapping_is_exhaustive_and_bounded() {
        assert_eq!(
            code_relationship_label(&RelationshipKind::Contains),
            "CONTAINS"
        );
        assert_eq!(code_relationship_label(&RelationshipKind::Calls), "CALLS");
        assert_eq!(bounded_limit(0), 1);
        assert_eq!(bounded_limit(10_000), 1_000);
        assert_eq!(
            stable_merge_key("entity", "workspace", "snapshot").unwrap(),
            "workspace:snapshot:entity"
        );
        assert!(stable_merge_key("", "workspace", "snapshot").is_err());
    }
}

#[cfg(test)]
mod level_two_real_storage_tests {
    use super::FalkorDbStorage;
    use crate::engines::coie::model::{CodeGraph, EntityKind, GraphEntity, ResolutionStatus};
    use crate::graph_storage::GraphStorage;
    use crate::source::RepositoryContext;

    #[tokio::test]
    #[ignore = "requires FalkorDB; run with --ignored"]
    async fn module_round_trip_persists_and_retrieves_a_code_entity() {
        let url = std::env::var("FALKORDB_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".into());
        let graph_name = format!("codemind_level2_storage_{}", uuid::Uuid::new_v4().simple());
        let storage = FalkorDbStorage::connect(&url, &graph_name).await.unwrap();
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let entity = GraphEntity {
            id: "symbol".into(),
            kind: EntityKind::Symbol,
            name: "target".into(),
            path: "src/lib.rs".into(),
            line: 1,
            context: context.clone(),
            language: Some(crate::source::Language::Rust),
            native_kind: Some("function".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: Some(crate::engines::coie::model::SourceEvidence {
                path: "src/lib.rs".into(),
                start_line: 1,
                end_line: 1,
                snapshot_id: "snapshot".into(),
                provenance: "test".into(),
            }),
        };
        storage
            .write_code_graph(&CodeGraph {
                context: context.clone(),
                entities: vec![entity.clone()],
                relationships: Vec::new(),
                diagnostics: Vec::new(),
            })
            .await
            .unwrap();
        assert_eq!(
            storage
                .entity_for(&context, &entity.id)
                .await
                .unwrap()
                .unwrap()
                .name,
            "target"
        );
    }
}
