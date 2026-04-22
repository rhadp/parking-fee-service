pub mod adapter;
pub mod config;
pub mod grpc;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod state;

pub mod proto {
    tonic::include_proto!("update_service.v1");
}

use grpc::GrpcUpdateService;
use podman::RealPodmanExecutor;
use proto::update_service_server::UpdateServiceServer;
use state::StateManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    // Initialise structured logging.
    tracing_subscriber::fmt::init();

    // Load configuration (REQ-7.1, REQ-7.2).
    let config_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());
    let cfg = match config::load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            // REQ-7.E2: invalid JSON → exit non-zero.
            tracing::error!("failed to load config from {config_path}: {e}");
            std::process::exit(1);
        }
    };

    // Log configuration and ready message (REQ-10.1).
    tracing::info!(
        grpc_port = cfg.grpc_port,
        inactivity_timeout_secs = cfg.inactivity_timeout_secs,
        container_storage_path = %cfg.container_storage_path,
        "update-service configuration loaded"
    );

    // Create shared components.
    let (event_tx, _event_rx) = broadcast::channel::<adapter::AdapterStateEvent>(256);
    let state_manager = Arc::new(StateManager::new(event_tx.clone()));
    let podman = Arc::new(RealPodmanExecutor);

    // Spawn the offload timer background task (REQ-6.1).
    let offload_sm = state_manager.clone();
    let offload_pm = podman.clone();
    let offload_tx = event_tx.clone();
    let inactivity_timeout = Duration::from_secs(cfg.inactivity_timeout_secs);
    tokio::spawn(async move {
        offload::start_offload_timer(
            offload_sm,
            offload_pm,
            offload_tx,
            inactivity_timeout,
            Duration::from_secs(60),
        )
        .await;
    });

    // Build the gRPC service.
    let svc = GrpcUpdateService::new(state_manager, podman, event_tx);

    // Bind address (REQ-7.3).
    let addr = format!("0.0.0.0:{}", cfg.grpc_port)
        .parse()
        .expect("invalid listen address");

    tracing::info!(%addr, "update-service ready, listening for gRPC connections");

    // Start the tonic server with graceful shutdown (REQ-10.2).
    let server = tonic::transport::Server::builder()
        .add_service(UpdateServiceServer::new(svc))
        .serve_with_shutdown(addr, shutdown_signal());

    if let Err(e) = server.await {
        tracing::error!("gRPC server error: {e}");
        std::process::exit(1);
    }
}

/// Wait for SIGTERM or SIGINT, then allow a 10-second drain window
/// before force-terminating (REQ-10.2, REQ-10.E1).
async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        )
        .expect("failed to install SIGTERM handler");

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("received SIGINT, initiating graceful shutdown");
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM, initiating graceful shutdown");
            }
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
        tracing::info!("received SIGINT, initiating graceful shutdown");
    }

    // REQ-10.E1: force-terminate after 10 seconds if in-flight RPCs
    // have not completed. tonic's `serve_with_shutdown` handles the
    // drain; we spawn a backstop timer that exits the process.
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(10)).await;
        tracing::warn!("shutdown timeout reached, force-terminating");
        std::process::exit(0);
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
