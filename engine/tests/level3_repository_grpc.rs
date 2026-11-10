//! Black-box RepositoryService test over a real local CodeMind checkout.
//! The test checks protobuf validation, authoritative server context, shared
//! repository lifecycle and durable graph publication.

mod support;

use codemind_engine::{
    contract::generated::{
        self, repository_service_client::RepositoryServiceClient,
        repository_service_server::RepositoryServiceServer,
    },
    engines::history::HistoryGraphContext,
    graph_storage::{GraphStorage, generation_id_for_context},
    transport::server::RepositoryServer,
};
use serde_json::Value;
use std::sync::Arc;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn repository_grpc_returns_authoritative_verified_knowledge_for_local_checkout() {
    let root = support::codemind_repository_root();
    let run_id = support::unique_run_id();
    let workspace_id = format!("level3-repository-grpc-{run_id}");
    let identity_snapshot = support::local_snapshot(&root, &workspace_id).await;
    let selected_target = support::unique_test_targeted_symbol(&identity_snapshot);
    let storage = Arc::new(support::connect_falkordb().await);
    let manager = support::repository_manager(Arc::clone(&storage));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(RepositoryServiceServer::new(RepositoryServer::new(manager)))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("RepositoryService server must stay available for the request");
    });

    let mut client = RepositoryServiceClient::connect(format!("http://{address}"))
        .await
        .expect("connect to in-process RepositoryService");
    let response = client
        .resolve_repository(generated::ResolveRepositoryRequest {
            source_path: root.to_string_lossy().into_owned(),
            context: Some(generated::RepositoryContext {
                repository_id: "untrusted-caller-repository-id".into(),
                workspace_id: workspace_id.clone(),
                source_snapshot_id: "untrusted-caller-snapshot-id".into(),
                graph_version: "untrusted-caller-graph-version".into(),
            }),
            correlation: Some(generated::Correlation {
                request_id: format!("repository-rpc-request-{run_id}"),
                trace_id: format!("repository-rpc-trace-{run_id}"),
                operation_id: format!("repository-rpc-correlation-{run_id}"),
            }),
        })
        .await
        .expect("RepositoryService must resolve the local checkout")
        .into_inner();

    let snapshot = match response
        .result
        .expect("RepositoryService must return a result")
    {
        generated::resolve_repository_response::Result::Snapshot(snapshot) => snapshot,
        generated::resolve_repository_response::Result::Error(error) => {
            panic!(
                "RepositoryService returned an error result: {}",
                error.message
            );
        }
    };
    let context = snapshot
        .context
        .expect("server must return its authoritative context");
    assert_eq!(context.workspace_id, workspace_id);
    assert_ne!(context.repository_id, "untrusted-caller-repository-id");
    assert_ne!(context.source_snapshot_id, "untrusted-caller-snapshot-id");
    assert_ne!(context.graph_version, "untrusted-caller-graph-version");

    let manifest: Value = serde_json::from_str(&snapshot.manifest_json)
        .expect("repository response manifest must be valid JSON");
    let repository_knowledge = manifest
        .get("repository_knowledge")
        .and_then(Value::as_object)
        .expect("response must include repository_knowledge metadata");
    assert_eq!(
        repository_knowledge
            .get("repository_id")
            .and_then(Value::as_str),
        Some(context.repository_id.as_str())
    );
    assert_eq!(
        repository_knowledge
            .get("source_snapshot_id")
            .and_then(Value::as_str),
        Some(context.source_snapshot_id.as_str())
    );
    assert_eq!(
        repository_knowledge
            .get("graph_version")
            .and_then(Value::as_str),
        Some(context.graph_version.as_str())
    );

    let domain_context = codemind_engine::source::RepositoryContext {
        repository_id: context.repository_id,
        workspace_id: context.workspace_id,
        source_snapshot_id: context.source_snapshot_id,
        graph_version: context.graph_version,
    };
    let generation_id = generation_id_for_context(&domain_context);
    assert!(
        storage
            .verify_published_generation(&domain_context, &generation_id)
            .await
            .unwrap()
    );
    assert!(
        storage
            .verified_snapshot_exists(&domain_context)
            .await
            .unwrap()
    );
    let history_context = HistoryGraphContext::from_source_context(&domain_context).unwrap();
    let selected_entities = storage
        .exact_code_entities_by_name(&domain_context, &selected_target.name)
        .await
        .unwrap();
    let selected_entity = selected_entities
        .iter()
        .find(|entity| entity.path == selected_target.path)
        .expect("the selected CodeMind symbol must exist in the current persisted projection");
    support::assert_random_sha256_id_contract(
        &selected_entity.id,
        "coie-entity:",
        "Persisted CoIE entity",
    );
    let stored_tests = storage
        .tests_for_context(
            &domain_context,
            std::slice::from_ref(&selected_entity.id),
            5_000,
        )
        .await
        .unwrap();
    assert!(
        !stored_tests.tests.is_empty(),
        "RepositoryService indexing must persist the selected target's TIE tests"
    );
    support::assert_tie_identity_contract(&stored_tests.tests, &stored_tests.relationships);
    let history_entities = storage
        .history_entities_for(&history_context, 2_000)
        .await
        .unwrap();
    assert!(!history_entities.is_empty());
    support::assert_history_graph_identity_contract(&history_entities, &[]);

    let manifest_path = support::persist_run_manifest(
        &run_id,
        &serde_json::json!({
            "run_id": run_id.clone(),
            "repository_root": root.display().to_string(),
            "workspace_id": domain_context.workspace_id.clone(),
            "physical_falkordb_graph": support::physical_graph_name(&domain_context.workspace_id),
            "repository_id": domain_context.repository_id.clone(),
            "source_snapshot_id": domain_context.source_snapshot_id.clone(),
            "graph_version": domain_context.graph_version.clone(),
            "transport": "RepositoryService gRPC",
            "manifest_file_count": manifest.get("files").and_then(Value::as_array).map_or(0, Vec::len),
            "history_entity_count": storage.history_entities_for(&history_context, 2_000).await.unwrap().len(),
        }),
    );

    server.abort();
    println!("RepositoryService resolved {}", root.display());
    println!("Run manifest: {}", manifest_path.display());
}
