//! UPDATE_SERVICE entry point.
//!
//! Loads configuration, initialises the state manager and podman executor,
//! spawns the offload timer, starts the tonic gRPC server, and handles
//! graceful shutdown on SIGTERM / SIGINT (07-REQ-10.1, 07-REQ-10.2).

use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use update_service::{
    config,
    grpc::{proto::update_service_server::UpdateServiceServer, GrpcUpdateService},
    podman::{run_offload_timer, RealPodmanExecutor, UpdateServiceImpl},
    state::StateManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise structured logging.
    tracing_subscriber::fmt::init();

    // ── Load configuration ────────────────────────────────────────────────────

    let config_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());
    let cfg = match config::load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to load configuration from '{config_path}': {e}");
            std::process::exit(1);
        }
    };

    info!(
        port = cfg.grpc_port,
        inactivity_timeout_secs = cfg.inactivity_timeout_secs,
        "update-service starting"
    );

    // ── Build shared infrastructure ───────────────────────────────────────────

    // Broadcast channel for state-change events (fan-out to all WatchAdapterStates subscribers).
    let (tx, _initial_rx) = tokio::sync::broadcast::channel(256);

    let state = Arc::new(StateManager::new(tx.clone()));
    let podman = Arc::new(RealPodmanExecutor);

    let service_impl = Arc::new(UpdateServiceImpl::new(
        Arc::clone(&state),
        Arc::clone(&podman),
        tx.clone(),
    ));

    // ── Spawn offload timer ───────────────────────────────────────────────────

    let inactivity_timeout = Duration::from_secs(cfg.inactivity_timeout_secs);
    // Check for stale adapters every 60 seconds.
    let check_interval = Duration::from_secs(60);

    tokio::spawn(run_offload_timer(
        Arc::clone(&state),
        Arc::clone(&podman),
        inactivity_timeout,
        check_interval,
    ));

    // ── Build gRPC server ─────────────────────────────────────────────────────

    let grpc_service = GrpcUpdateService::new(Arc::clone(&service_impl), tx.clone());
    let grpc_server = UpdateServiceServer::new(grpc_service);

    let addr: std::net::SocketAddr = format!("0.0.0.0:{}", cfg.grpc_port).parse()?;

    info!("update-service ready on {addr}");

    // ── Graceful shutdown ─────────────────────────────────────────────────────
    //
    // Design:
    //   1. Spawn the tonic server as a background task, passing a oneshot
    //      receiver as its shutdown signal.
    //   2. Wait for SIGTERM / SIGINT in main.
    //   3. Send the shutdown signal so tonic stops accepting new connections.
    //   4. Wait up to 10 seconds for in-flight RPCs to drain (07-REQ-10.E1).

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(grpc_server)
            .serve_with_shutdown(addr, async move {
                let _ = shutdown_rx.await;
                info!("Shutdown signal received; draining in-flight RPCs");
            }),
    );

    // Block until we receive an OS termination signal.
    shutdown_signal().await;

    // Signal the gRPC server to stop accepting new requests.
    let _ = shutdown_tx.send(());

    // Allow up to 10 seconds for in-flight RPCs to complete (07-REQ-10.E1).
    match tokio::time::timeout(Duration::from_secs(10), server_handle).await {
        Ok(Ok(Ok(()))) => info!("update-service shutdown complete"),
        Ok(Ok(Err(e))) => tracing::error!("gRPC server error during shutdown: {e}"),
        Ok(Err(e)) => tracing::error!("Server task error: {e}"),
        Err(_) => info!("Shutdown drain timeout (10 s); force-terminating"),
    }

    Ok(())
}

/// Wait for SIGTERM or SIGINT (CTRL-C).
#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate())
        .expect("failed to register SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt())
        .expect("failed to register SIGINT handler");

    tokio::select! {
        biased;
        _ = sigterm.recv() => {
            info!("Received SIGTERM");
        }
        _ = sigint.recv() => {
            info!("Received SIGINT");
        }
    }
}

/// Fallback for non-Unix platforms: wait for Ctrl-C only.
#[cfg(not(unix))]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to register Ctrl+C handler");
    info!("Received Ctrl+C");
}

#[cfg(test)]
mod tests {
    /// Verifies the crate compiles successfully (01-REQ-8.1, TS-01-26).
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
