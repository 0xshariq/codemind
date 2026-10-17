pub mod conversions;
pub mod progress;
pub mod server;

#[cfg(test)]
mod integration_tests {
    use super::conversions;
    use crate::contract::result::{CanonicalResult, Completeness, Correlation, Freshness, Outcome};
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
    fn canonical_result_conversion_validates_metadata_before_serialization() {
        let result = CanonicalResult::new(
            context(),
            Correlation {
                request_id: "request".into(),
                trace_id: "trace".into(),
                operation_id: "operation".into(),
            },
            Outcome::Success,
            Completeness::Complete,
            Freshness::Current,
            vec!["entity".to_owned()],
        );
        let json = conversions::canonical_json(&result).unwrap();
        assert!(json.contains("\"request_id\":\"request\""));
        assert!(json.contains("\"source_snapshot_id\":\"snapshot\""));
    }
}

#[cfg(test)]
mod level_two_server_tests {
    use super::server::RepositoryServer;
    use crate::contract::generated::{self, repository_service_server::RepositoryService};
    use std::fs;
    use tempfile::tempdir;
    use tonic::Request;

    #[tokio::test]
    async fn repository_server_uses_the_source_boundary_and_returns_a_real_snapshot() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("lib.rs"), "fn target() {}\n").unwrap();
        let response = RepositoryService::resolve_repository(
            &RepositoryServer::default(),
            Request::new(generated::ResolveRepositoryRequest {
                source_path: root.path().to_string_lossy().into_owned(),
                context: Some(generated::RepositoryContext {
                    repository_id: "repo".into(),
                    workspace_id: "workspace".into(),
                    source_snapshot_id: "request-snapshot".into(),
                    graph_version: "request-graph".into(),
                }),
                correlation: None,
            }),
        )
        .await
        .unwrap()
        .into_inner();
        let snapshot = match response.result.unwrap() {
            generated::resolve_repository_response::Result::Snapshot(snapshot) => snapshot,
            generated::resolve_repository_response::Result::Error(error) => {
                panic!("repository resolution failed: {}", error.message);
            }
        };
        assert!(snapshot.manifest_json.contains("lib.rs"));
        assert!(!snapshot.context.unwrap().source_snapshot_id.is_empty());
    }
}
