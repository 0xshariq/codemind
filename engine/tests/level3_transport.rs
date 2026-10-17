mod support;

use codemind_engine::{
    contract::generated::{
        self, repository_service_client::RepositoryServiceClient,
        repository_service_server::RepositoryServiceServer,
    },
    transport::server::RepositoryServer,
};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

#[tokio::test]
#[ignore = "requires GitHub access; run with --ignored"]
async fn repository_grpc_service_reads_the_github_repository_boundary() {
    let (_temp, root) = support::fetch_github_repository();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(RepositoryServiceServer::new(RepositoryServer::default()))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    let mut client = RepositoryServiceClient::connect(format!("http://{address}"))
        .await
        .unwrap();
    let response = client
        .resolve_repository(generated::ResolveRepositoryRequest {
            source_path: root.to_string_lossy().into_owned(),
            context: Some(generated::RepositoryContext {
                repository_id: support::github_repository_url(),
                workspace_id: "level3-grpc".into(),
                source_snapshot_id: "request-snapshot-placeholder".into(),
                graph_version: "request-graph-placeholder".into(),
            }),
            correlation: Some(generated::Correlation {
                request_id: "grpc-request".into(),
                trace_id: "grpc-trace".into(),
                operation_id: "grpc-operation".into(),
            }),
        })
        .await
        .unwrap()
        .into_inner();
    let snapshot = match response
        .result
        .expect("repository RPC must return a result")
    {
        generated::resolve_repository_response::Result::Snapshot(snapshot) => snapshot,
        generated::resolve_repository_response::Result::Error(error) => {
            panic!("repository RPC failed: {}", error.message);
        }
    };
    assert!(!snapshot.manifest_json.is_empty());
    assert!(!snapshot.context.unwrap().source_snapshot_id.is_empty());
    server.abort();
}
