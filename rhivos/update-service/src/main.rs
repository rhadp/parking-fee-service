pub mod adapter;
pub mod config;
pub mod grpc;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod state;

#[cfg(test)]
mod proptest_cases;

#[cfg(test)]
mod tests_install;

#[cfg(test)]
mod tests_lifecycle;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tonic::transport::Server;

use crate::adapter::AdapterStateEvent;
use crate::config::load_config;
use crate::grpc::proto::update_service_server::UpdateServiceServer;
use crate::grpc::UpdateServiceImpl;
use crate::offload::run_offload_timer;
use crate::podman::RealPodmanExecutor;
use crate::state::StateManager;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());
    let config = match load_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!("Failed to load configuration from {config_path}: {e}");
            std::process::exit(1);
        }
    };

    // Log configuration at startup (07-REQ-10.1)
    tracing::info!(
        grpc_port = config.grpc_port,
        inactivity_timeout_secs = config.inactivity_timeout_secs,
        registry_url = %config.registry_url,
        container_storage_path = %config.container_storage_path,
        "UPDATE_SERVICE configuration loaded"
    );

    // Create shared infrastructure
    let (broadcast_tx, _broadcast_rx) = broadcast::channel::<AdapterStateEvent>(256);
    let state_mgr = Arc::new(StateManager::new(broadcast_tx.clone()));
    let podman: Arc<dyn crate::podman::PodmanExecutor> = Arc::new(RealPodmanExecutor);

    // Spawn the offload timer background task (07-REQ-6.1)
    let offload_state = state_mgr.clone();
    let offload_podman = podman.clone();
    let inactivity_timeout = Duration::from_secs(config.inactivity_timeout_secs);
    let check_interval = Duration::from_secs(60);
    tokio::spawn(async move {
        run_offload_timer(offload_state, offload_podman, inactivity_timeout, check_interval).await;
    });

    // Build the gRPC service
    let update_service = UpdateServiceImpl::new(state_mgr, podman, broadcast_tx);
    let svc = UpdateServiceServer::new(update_service);

    // Parse the listen address (07-REQ-7.3)
    let addr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .expect("invalid listen address");

    // Register signal handlers BEFORE logging ready so that signals received
    // immediately after the ready message are handled gracefully (07-REQ-10.2).
    #[cfg(unix)]
    let mut sigterm = {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::terminate()).expect("failed to register SIGTERM handler")
    };
    #[cfg(unix)]
    let mut sigint = {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::interrupt()).expect("failed to register SIGINT handler")
    };

    tracing::info!(%addr, "UPDATE_SERVICE ready");

    // Set up graceful shutdown with 10-second drain timeout (07-REQ-10.2, 07-REQ-10.E1)
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        // Wait for SIGTERM or SIGINT
        #[cfg(unix)]
        {
            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("received SIGTERM, initiating graceful shutdown");
                }
                _ = sigint.recv() => {
                    tracing::info!("received SIGINT, initiating graceful shutdown");
                }
            }
        }

        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to register Ctrl-C handler");
            tracing::info!("received Ctrl-C, initiating graceful shutdown");
        }

        // Signal tonic to stop accepting new RPCs and drain in-flight ones
        let _ = shutdown_tx.send(());

        // If in-flight RPCs don't complete within 10 seconds, force-terminate (07-REQ-10.E1)
        tokio::time::sleep(Duration::from_secs(10)).await;
        tracing::warn!(
            "in-flight RPCs did not complete within 10 seconds, force-terminating"
        );
        std::process::exit(0);
    });

    // Start serving
    if let Err(e) = Server::builder()
        .add_service(svc)
        .serve_with_shutdown(addr, async {
            shutdown_rx.await.ok();
        })
        .await
    {
        tracing::error!("gRPC server error: {e}");
        std::process::exit(1);
    }

    tracing::info!("UPDATE_SERVICE shut down cleanly");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Verify the binary crate compiles successfully.
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }
}
