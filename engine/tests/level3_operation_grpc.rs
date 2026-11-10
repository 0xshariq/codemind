//! Black-box OperationService test: real CodeMind checkout -> repository
//! indexing -> verified ChIE investigation -> streamed persisted events.

mod support;

use codemind_engine::{
    contract::{
        events::EngineEvent,
        generated::{
            self, operation_service_client::OperationServiceClient,
            operation_service_server::OperationServiceServer,
        },
        operation::OperationState,
    },
    engines::history::HistoryGraphContext,
    graph_storage::GraphStorage,
    transport::server::OperationServer,
};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn operation_grpc_indexes_and_investigates_the_real_codemind_checkout() {
    let root = support::codemind_repository_root();
    let run_id = support::unique_run_id();
    let workspace_id = format!("level3-operation-grpc-{run_id}");
    let snapshot = support::local_snapshot(&root, &workspace_id).await;
    let selected_target = support::unique_test_targeted_symbol(&snapshot);
    let storage = Arc::new(support::connect_falkordb().await);
    let manager = support::repository_manager(Arc::clone(&storage));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let operation_server = OperationServer::new(Arc::clone(&storage), manager);
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(OperationServiceServer::new(operation_server))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("OperationService server must remain healthy during the request");
    });

    let mut client = OperationServiceClient::connect(format!("http://{address}"))
        .await
        .expect("connect to the in-process OperationService endpoint");
    let response = client
        .start_investigation(generated::StartInvestigationRequest {
            context: Some(generated::RepositoryContext {
                repository_id: snapshot.context.repository_id.clone(),
                workspace_id: workspace_id.clone(),
                source_snapshot_id: snapshot.context.source_snapshot_id.clone(),
                graph_version: snapshot.context.graph_version.clone(),
            }),
            source_path: root.to_string_lossy().into_owned(),
            symbol_name: selected_target.name.clone(),
            correlation: Some(generated::Correlation {
                request_id: format!("level3-operation-request-{run_id}"),
                trace_id: format!("level3-operation-trace-{run_id}"),
                operation_id: format!("level3-operation-client-correlation-{run_id}"),
            }),
        })
        .await
        .expect("StartInvestigation must accept the real local repository")
        .into_inner();

    let operation_id = response.operation_id;
    assert!(!operation_id.is_empty());
    let proto_context = response
        .context
        .expect("server must return its resolved context");
    assert_eq!(proto_context.workspace_id, workspace_id);
    assert_eq!(proto_context.repository_id, snapshot.context.repository_id);
    assert!(!response.selected_entity_id.is_empty());

    let mut stream = client
        .stream_progress(generated::StreamProgressRequest {
            context: Some(proto_context.clone()),
            operation_id: operation_id.clone(),
            after_sequence: 0,
            limit: 1_000,
        })
        .await
        .expect("StreamProgress must accept the returned operation context")
        .into_inner();
    let mut progress = Vec::new();
    tokio::time::timeout(Duration::from_secs(180), async {
        while let Some(event) = stream
            .message()
            .await
            .expect("progress stream must be readable")
        {
            progress.push(event);
        }
    })
    .await
    .expect("operation progress stream did not reach its terminal state within 180 seconds");

    assert!(!progress.is_empty(), "operation stream returned no events");
    assert!(
        progress
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence)
    );
    assert!(
        progress
            .iter()
            .any(|event| event.stage == "repository-index.coie")
    );
    assert!(
        progress
            .iter()
            .any(|event| event.stage == "repository-index.history")
    );
    assert!(
        progress
            .iter()
            .any(|event| event.stage == "repository-index.tie")
    );
    assert!(
        progress
            .iter()
            .any(|event| event.stage == "repository-index.verify")
    );
    assert!(progress.iter().any(|event| event.stage == "chie"));
    assert!(
        progress
            .iter()
            .any(|event| event.message == "stage completed")
    );

    let context = codemind_engine::source::RepositoryContext {
        repository_id: proto_context.repository_id,
        workspace_id: proto_context.workspace_id,
        source_snapshot_id: proto_context.source_snapshot_id,
        graph_version: proto_context.graph_version,
    };
    let stored_operation = storage
        .retrieve_operation(&context, &operation_id)
        .await
        .unwrap();
    assert_eq!(stored_operation.state, OperationState::Completed);
    let result_reference = stored_operation
        .result_reference
        .as_deref()
        .expect("completed operation must reference its persisted analysis result");
    let analysis_run_id = result_reference
        .strip_prefix("change-analysis:")
        .expect("operation result must use a change-analysis reference");
    let change_relationships = storage
        .derived_relationships_for_run(&context, analysis_run_id, 2_000)
        .await
        .unwrap();
    assert!(
        !change_relationships.is_empty(),
        "OperationService did not persist the Change Graph"
    );
    let stored_events = storage
        .events_since(&context, &operation_id, 0, 1_000)
        .await
        .unwrap();
    assert!(
        stored_events
            .iter()
            .any(|event| matches!(event, EngineEvent::Completed { .. }))
    );

    let selected = storage
        .entity_for(&context, &response.selected_entity_id)
        .await
        .unwrap();
    assert!(
        selected.is_some(),
        "OperationService returned a selected entity absent from Code Graph"
    );
    let history_context = HistoryGraphContext::from_source_context(&context).unwrap();
    assert!(
        !storage
            .history_entities_for(&history_context, 2_000)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        !storage
            .tests_for_context(
                &context,
                std::slice::from_ref(&response.selected_entity_id),
                500
            )
            .await
            .unwrap()
            .tests
            .is_empty()
    );

    let graph_name = support::physical_graph_name(&workspace_id);
    let manifest_path = support::persist_run_manifest(
        &run_id,
        &json!({
            "run_id": run_id.clone(),
            "repository_root": root.display().to_string(),
            "workspace_id": workspace_id.clone(),
            "physical_falkordb_graph": graph_name.clone(),
            "repository_id": context.repository_id.clone(),
            "source_snapshot_id": context.source_snapshot_id.clone(),
            "graph_version": context.graph_version.clone(),
            "selected_symbol": selected_target.name,
            "selected_entity_id": response.selected_entity_id,
            "operation_id": operation_id,
            "transport": "OperationService gRPC",
            "progress_event_count": progress.len(),
            "stored_event_count": stored_events.len(),
            "change_relationship_count": change_relationships.len(),
            "analysis_run_id": analysis_run_id,
            "terminal_state": "completed",
        }),
    );

    server.abort();
    println!("OperationService Level 3 run persisted in {graph_name}");
    println!("Run manifest: {}", manifest_path.display());
}
