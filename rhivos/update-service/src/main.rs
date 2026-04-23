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

    // Install signal handlers BEFORE logging "ready" so the process can
    // handle SIGTERM/SIGINT immediately after the ready message appears.
    // Signal handlers must be created synchronously (not inside an async
    // fn body, which only runs when polled) to prevent a race where
    // SIGTERM arrives between the "ready" log and handler registration
    // (REQ-10.2, REQ-10.E1).
    let shutdown = make_shutdown_signal();

    tracing::info!(%addr, "update-service ready, listening for gRPC connections");

    // Start the tonic server with graceful shutdown (REQ-10.2).
    let server = tonic::transport::Server::builder()
        .add_service(UpdateServiceServer::new(svc))
        .serve_with_shutdown(addr, shutdown);

    if let Err(e) = server.await {
        tracing::error!("gRPC server error: {e}");
        std::process::exit(1);
    }
}

/// Create a future that resolves when SIGTERM or SIGINT is received.
///
/// Signal handlers are installed eagerly (when this function is called),
/// not lazily (when the returned future is first polled). This is crucial
/// because `async fn` bodies don't execute until polled, so
/// `tokio::signal::unix::signal()` would not register the OS handler
/// until the tokio runtime schedules the task — leaving a window where
/// signals use the default (terminate) action.
///
/// After the signal fires, a 10-second backstop timer is started: if
/// in-flight RPCs have not drained by then, the process force-exits
/// with code 0 (REQ-10.E1).
fn make_shutdown_signal() -> impl std::future::Future<Output = ()> {
    // Install OS signal handlers NOW, synchronously. Using
    // `tokio::signal::unix::signal()` instead of `tokio::signal::ctrl_c()`
    // because `ctrl_c()` installs its handler lazily when polled, whereas
    // `signal()` installs the handler immediately on construction.
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    )
    .expect("failed to install SIGTERM handler");

    #[cfg(unix)]
    let mut sigint = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::interrupt(),
    )
    .expect("failed to install SIGINT handler");

    async move {
        #[cfg(unix)]
        {
            tokio::select! {
                _ = sigint.recv() => {
                    tracing::info!("received SIGINT, initiating graceful shutdown");
                }
                _ = sigterm.recv() => {
                    tracing::info!("received SIGTERM, initiating graceful shutdown");
                }
            }
        }

        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await.ok();
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
