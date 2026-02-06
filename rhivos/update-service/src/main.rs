//! UPDATE_SERVICE - Container Lifecycle Management
//!
//! This service runs in the RHIVOS QM partition and handles
//! dynamic adapter installation and lifecycle management.
//!
//! Communication:
//! - Receives commands from PARKING_APP via gRPC/TLS
//! - Downloads adapters from REGISTRY via HTTPS/OCI

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::watch;
use tonic::transport::Server;
use tracing::info;

use update_service::authenticator::{RegistryAuthenticator, RegistryCredentials};
use update_service::config::ServiceConfig;
use update_service::logger::{init_tracing, OperationLogger};
use update_service::offload::OffloadScheduler;
use update_service::proto::update_service_server::UpdateServiceServer;
use update_service::service::UpdateServiceImpl;
use update_service::watcher::WatcherManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = ServiceConfig::from_env();

    // Initialize tracing
    init_tracing(&config.log_level);

    info!("Starting UPDATE_SERVICE...");
    info!("Listen address: {}", config.listen_addr);
    info!("Storage path: {}", config.storage_path);

    // Create shared components
    let logger = Arc::new(OperationLogger::new("update-service"));
    let watcher_manager = Arc::new(WatcherManager::new());

    // Create authenticator with optional credentials
    let credentials = if config.has_credentials() {
        Some(RegistryCredentials::new(
            config.registry_username.clone().unwrap(),
            config.registry_password.clone().unwrap(),
        ))
    } else {
        info!("No registry credentials configured, using anonymous access");
        None
    };

    let authenticator = Arc::new(RegistryAuthenticator::new(
        credentials,
        config.token_refresh_buffer_secs,
        logger.clone(),
    ));

    // Create service
    let service = UpdateServiceImpl::new(
        config.clone(),
        authenticator,
        watcher_manager.clone(),
        logger.clone(),
    );

    // Restore state from running containers
    if let Err(e) = service.restore_state().await {
        info!("Note: Could not restore state from containers: {}", e);
        // Non-fatal - may not have podman available in dev
    }

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Start offload scheduler
    let state_tracker = service.state_tracker();
    let container_manager = service.container_manager();
    let mut offload_scheduler = OffloadScheduler::new(
        state_tracker,
        container_manager,
        config.offload_threshold_hours,
        config.offload_check_interval_minutes,
        logger.clone(),
        shutdown_rx.clone(),
    );

    let offload_handle = tokio::spawn(async move {
        offload_scheduler.run().await;
    });

    // Parse listen address
    let addr: SocketAddr = config.listen_addr.parse()?;

    info!("UPDATE_SERVICE listening on {}", addr);

    // Create gRPC server
    let grpc_server = Server::builder()
        .add_service(UpdateServiceServer::new(service))
        .serve_with_shutdown(addr, async move {
            tokio::signal::ctrl_c().await.ok();
            info!("Received shutdown signal");
            let _ = shutdown_tx.send(true);
        });

    // Run the server
    grpc_server.await?;

    // Wait for offload scheduler to complete
    offload_handle.await?;

    info!("UPDATE_SERVICE shutdown complete");

    Ok(())
}
