mod support;

use codemind_engine::{
    contract::generated::{
        self, operation_service_client::OperationServiceClient,
        operation_service_server::OperationServiceServer,
    },
    engines::coie::CoieEngine,
    graph_storage::GraphStorage,
    source::SourceProvider,
    transport::server::OperationServer,
};
use std::sync::Arc;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires GitHub access and FalkorDB; run with --ignored"]
async fn operation_grpc_starts_the_pipeline_and_streams_replayable_progress() {
    let Some((root, snapshot)) = support::github_snapshot() else {
        return;
    };
    let graph = CoieEngine.extract(&snapshot).unwrap();
    let symbol_name = support::first_callable_symbol_name(&graph);
    let graph_name = format!("codemind_level3_grpc_{}", Uuid::new_v4().simple());
    let storage = Arc::new(support::storage(&graph_name).await);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let operation_server = OperationServer::new(Arc::clone(&storage), SourceProvider::default());

    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(OperationServiceServer::new(operation_server))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    let mut client = OperationServiceClient::connect(format!("http://{address}"))
        .await
        .unwrap();

    let started = client
        .start_investigation(generated::StartInvestigationRequest {
            context: Some(generated::RepositoryContext {
                repository_id: support::github_repository_url(),
                workspace_id: "level3-operation".into(),
                source_snapshot_id: "request-snapshot-placeholder".into(),
                graph_version: "request-graph-placeholder".into(),
            }),
            source_path: root.to_string_lossy().into_owned(),
            symbol_name,
            correlation: Some(generated::Correlation {
                request_id: "operation-grpc-request".into(),
                trace_id: "operation-grpc-trace".into(),
                operation_id: "operation-grpc-correlation".into(),
            }),
        })
        .await
        .unwrap()
        .into_inner();

    let operation_id = started.operation_id.clone();
    let context = started.context.unwrap();

    let mut stream = client
        .stream_progress(generated::StreamProgressRequest {
            context: Some(context.clone()),
            operation_id: operation_id.clone(),
            after_sequence: 0,
            limit: 200,
        })
        .await
        .unwrap()
        .into_inner();

    let mut events = Vec::new();

    while let Some(event) = stream.message().await.unwrap() {
        events.push(event);
    }

    assert!(!events.is_empty());

    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence)
    );

    assert!(events.iter().any(|event| event.stage == "coie"));
    assert!(events.iter().any(|event| event.stage == "tie"));
    assert!(events.iter().any(|event| event.stage == "chie"));
    assert!(
        events
            .iter()
            .any(|event| event.message == "stage completed")
    );

    let stored = storage
        .events_since(
            &codemind_engine::source::RepositoryContext {
                repository_id: context.repository_id,
                workspace_id: context.workspace_id,
                source_snapshot_id: context.source_snapshot_id,
                graph_version: context.graph_version,
            },
            &operation_id,
            0,
            200,
        )
        .await
        .unwrap();

    assert!(!stored.is_empty());

    server.abort();
}
