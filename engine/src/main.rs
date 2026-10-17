use anyhow::{Context, Result};
use codemind_engine::config::Config;
use codemind_engine::contract::generated::{
    intelligence_service_server::IntelligenceServiceServer,
    memory_service_server::MemoryServiceServer, operation_service_server::OperationServiceServer,
    repository_service_server::RepositoryServiceServer,
};
use codemind_engine::graph_storage::FalkorDbStorage;
use codemind_engine::transport::server::{
    IntelligenceServer, MemoryServer, OperationServer, RepositoryServer,
};
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let config = Config::from_env().context("failed to load CodeMind engine configuration")?;

    let storage = Arc::new(
        FalkorDbStorage::connect(&config.falkordb_url, &config.falkordb_graph)
            .await
            .context("failed to connect to FalkorDB")?,
    );

    info!(
        address = %config.engine_addr,
        graph = %config.falkordb_graph,
        falkordb = %config.falkordb_url,
        "CodeMind Rust engine connected to FalkorDB"
    );

    info!("CodeMind gRPC service is ready for Agent Runtime connections");

    let shutdown = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to listen for Ctrl+C");
            return;
        }

        info!("shutdown signal received, stopping CodeMind Rust engine");
    };

    Server::builder()
        .add_service(IntelligenceServiceServer::new(IntelligenceServer::new(
            Arc::clone(&storage),
        )))
        .add_service(MemoryServiceServer::new(MemoryServer::new(Arc::clone(
            &storage,
        ))))
        .add_service(OperationServiceServer::new(OperationServer::new(
            Arc::clone(&storage),
            codemind_engine::source::SourceProvider::default(),
        )))
        .add_service(RepositoryServiceServer::new(RepositoryServer::default()))
        .serve_with_shutdown(config.engine_addr, shutdown)
        .await
        .context("CodeMind gRPC server stopped unexpectedly")?;

    info!("CodeMind Rust engine stopped");

    Ok(())
}
