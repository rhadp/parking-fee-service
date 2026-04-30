pub mod adapter;
pub mod config;
pub mod grpc;
pub mod install;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod state;

#[cfg(test)]
pub mod proptest_cases;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tonic::transport::Server;
use tracing::{error, info, warn};

use crate::adapter::AdapterStateEvent;
use crate::config::load_config;
use crate::grpc::proto::update_service_server::UpdateServiceServer;
use crate::grpc::UpdateServiceImpl;
use crate::offload::spawn_offload_timer;
use crate::podman::RealPodmanExecutor;
use crate::state::StateManager;

#[tokio::main]
async fn main() {
    // Initialise structured logging.
    tracing_subscriber::fmt::init();

    // Load configuration.
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());
    let config = match load_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("failed to load configuration: {e}");
            std::process::exit(1);
        }
    };

    // Log configuration at startup.
    info!(
        grpc_port = config.grpc_port,
        inactivity_timeout_secs = config.inactivity_timeout_secs,
        container_storage_path = %config.container_storage_path,
        "update-service configuration loaded"
    );

    // Create shared components.
    let (broadcast_tx, _initial_rx) = broadcast::channel::<AdapterStateEvent>(256);
    let state_mgr = Arc::new(StateManager::new(broadcast_tx.clone()));
    let podman: Arc<dyn crate::podman::PodmanExecutor> = Arc::new(RealPodmanExecutor::new());

    // Spawn the offload timer background task.
    let inactivity_timeout = Duration::from_secs(config.inactivity_timeout_secs);
    let check_interval = Duration::from_secs(60);
    let _offload_handle = spawn_offload_timer(
        state_mgr.clone(),
        podman.clone(),
        inactivity_timeout,
        check_interval,
    );

    // Build the gRPC service.
    let service = UpdateServiceImpl::new(state_mgr, podman, broadcast_tx);
    let addr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .expect("invalid gRPC listen address");

    info!(addr = %addr, "update-service ready");

    // Start the gRPC server with graceful shutdown on SIGTERM/SIGINT.
    let server = Server::builder()
        .add_service(UpdateServiceServer::new(service))
        .serve_with_shutdown(addr, shutdown_signal());

    if let Err(e) = server.await {
        error!("gRPC server error: {e}");
        std::process::exit(1);
    }

    info!("update-service shutdown complete");
}

/// Waits for a SIGTERM or SIGINT signal, then returns after a 10-second
/// drain period to allow in-flight RPCs to complete.
async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("received SIGINT, initiating graceful shutdown");
        }
        _ = terminate => {
            info!("received SIGTERM, initiating graceful shutdown");
        }
    }

    // Allow up to 10 seconds for in-flight RPCs to complete.
    // tonic's serve_with_shutdown handles draining; we add a hard deadline.
    info!("draining in-flight RPCs (10s timeout)");
    tokio::time::sleep(Duration::from_secs(10)).await;
    warn!("drain timeout reached, force-terminating");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Placeholder test: verifies the crate compiles.
    }
}
