use anyhow::Result;
use codemind_engine::config::Config;
use codemind_engine::contract::generated::intelligence_service_server::IntelligenceServiceServer;
use codemind_engine::graph_storage::FalkorDbStorage;
use codemind_engine::transport::server::IntelligenceServer;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = Config::from_env()?;
    let storage =
        Arc::new(FalkorDbStorage::connect(&config.falkordb_url, &config.falkordb_graph).await?);

    info!(
        address = %config.engine_addr,
        graph = %config.falkordb_graph,
        "codemind-engine foundation is ready"
    );

    Server::builder()
        .add_service(IntelligenceServiceServer::new(IntelligenceServer::new(
            storage,
        )))
        .serve(config.engine_addr)
        .await?;
    Ok(())
}
