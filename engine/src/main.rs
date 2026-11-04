use anyhow::{Context, Result};
use codemind_engine::config::Config;
use codemind_engine::contract::generated::{
    intelligence_service_server::IntelligenceServiceServer,
    memory_service_server::MemoryServiceServer, operation_service_server::OperationServiceServer,
    repository_service_server::RepositoryServiceServer,
};
use codemind_engine::git::GitCommandConfig;
use codemind_engine::graph_storage::FalkorDbStorage;
use codemind_engine::repository::{CodeMindPaths, RepositoryManager, RepositoryManagerOptions};
use codemind_engine::source::SourceProvider;
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

    // FALKORDB_GRAPH is a trusted namespace prefix. The storage adapter derives one
    // collision-resistant physical graph name from each validated workspace ID.
    let storage = Arc::new(
        FalkorDbStorage::connect_with_limits(
            &config.falkordb_url,
            &config.falkordb_graph,
            config.falkordb_query_timeout_ms,
            config.falkordb_max_inflight,
        )
        .await
        .context("failed to connect to FalkorDB")?,
    );

    let paths = CodeMindPaths::discover().context("failed to resolve ~/.codemind")?;
    let repository_manager = Arc::new(RepositoryManager::with_options(
        Arc::clone(&storage),
        paths,
        SourceProvider::default(),
        GitCommandConfig::default(),
        RepositoryManagerOptions {
            history_options: config.git_history_options,
            enable_incremental_mutation: config.enable_incremental_mutation,
        },
    ));

    // Reconcile operation-owned temporary workspaces before accepting requests.
    // Recovery is deliberately separate from graph-operation event replay and
    // never treats local metadata alone as proof that a graph is ready.
    let recovery = repository_manager
        .recover_startup()
        .await
        .context("failed to recover CodeMind temporary workspaces")?;

    info!(
        scanned = recovery.scanned,
        cleaned_verified = recovery.cleaned_verified,
        cleaned_abandoned = recovery.cleaned_abandoned,
        retained_for_retry = recovery.retained_for_retry,
        skipped_active = recovery.skipped_active,
        preserved_for_review = recovery.preserved_for_manual_review,
        "CodeMind workspace recovery completed"
    );

    // Repository and operation services deliberately share one lifecycle manager.
    // No transport path may create a second repository/source authority.
    let repository_server = RepositoryServer::new(Arc::clone(&repository_manager));
    let operation_server =
        OperationServer::new(Arc::clone(&storage), Arc::clone(&repository_manager));
    let memory_server = MemoryServer::new(Arc::clone(&storage));
    let intelligence_server = IntelligenceServer::new(Arc::clone(&storage));

    let addr = config.engine_addr;

    info!(%addr, "CodeMind engine listening");

    Server::builder()
        .add_service(RepositoryServiceServer::new(repository_server))
        .add_service(OperationServiceServer::new(operation_server))
        .add_service(MemoryServiceServer::new(memory_server))
        .add_service(IntelligenceServiceServer::new(intelligence_server))
        .serve(addr)
        .await
        .context("CodeMind gRPC server failed")?;

    Ok(())
}
