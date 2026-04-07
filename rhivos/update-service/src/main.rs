use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tonic::transport::Server;
use tracing::{error, info};

use update_service::adapter::AdapterStateEvent;
use update_service::config::load_config;
use update_service::grpc::GrpcUpdateService;
use update_service::offload::spawn_offload_timer;
use update_service::podman::RealPodmanExecutor;
use update_service::proto::update_service_server::UpdateServiceServer;
use update_service::state::StateManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());
    let config = match load_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load configuration from {config_path}: {e}");
            std::process::exit(1);
        }
    };

    let grpc_port = config.grpc_port;
    let inactivity_timeout_secs = config.inactivity_timeout_secs;

    info!(
        port = grpc_port,
        inactivity_timeout_secs = inactivity_timeout_secs,
        "UPDATE_SERVICE configuration loaded"
    );

    // Create shared components
    let (broadcast_tx, _) = broadcast::channel::<AdapterStateEvent>(256);
    let state_mgr = Arc::new(StateManager::new(broadcast_tx));
    let podman: Arc<dyn update_service::podman::PodmanExecutor> =
        Arc::new(RealPodmanExecutor::new());

    // Spawn offload timer background task
    let offload_handle = spawn_offload_timer(
        state_mgr.clone(),
        podman.clone(),
        Duration::from_secs(inactivity_timeout_secs),
        Duration::from_secs(60),
    );

    // Build gRPC service
    let grpc_service = GrpcUpdateService::new(state_mgr, podman, config);
    let service = UpdateServiceServer::new(grpc_service);

    let addr = format!("0.0.0.0:{grpc_port}").parse()?;

    info!("UPDATE_SERVICE ready, listening on {addr}");

    // Run server with graceful shutdown on SIGTERM/SIGINT
    let shutdown = async {
        let ctrl_c = tokio::signal::ctrl_c();
        #[cfg(unix)]
        {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to install SIGTERM handler");
            tokio::select! {
                _ = ctrl_c => {
                    info!("Received SIGINT, shutting down");
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, shutting down");
                }
            }
        }
        #[cfg(not(unix))]
        {
            ctrl_c.await.ok();
            info!("Received shutdown signal, shutting down");
        }
    };

    Server::builder()
        .add_service(service)
        .serve_with_shutdown(addr, shutdown)
        .await?;

    // Cancel offload timer
    offload_handle.abort();

    info!("UPDATE_SERVICE shut down cleanly");
    Ok(())
}
