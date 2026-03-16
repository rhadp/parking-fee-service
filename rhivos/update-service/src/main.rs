//! UPDATE_SERVICE — entry point.
//!
//! Loads configuration, wires up components, starts the offload timer, and
//! serves the gRPC API.  Handles SIGTERM/SIGINT by stopping running adapters
//! and gracefully shutting down the gRPC server (REQ-8.1, REQ-8.2).

use std::net::SocketAddr;
use std::sync::Arc;

use tracing::{error, info, warn};
use update_service::{
    config::load_config,
    container::{ContainerRuntime, PodmanRuntime},
    grpc_handler::UpdateServiceImpl,
    model::AdapterState,
    offload::spawn_offload_timer,
    proto::updateservice::update_service_server::UpdateServiceServer,
    state::StateManager,
};

fn print_usage() {
    println!(
        "update-service v{} - RHIVOS OCI adapter lifecycle manager",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Usage: update-service [command]");
    println!();
    println!("Commands:");
    println!("  serve    Start the update service gRPC server");
    println!();
    println!("Environment variables:");
    println!("  CONFIG_PATH    Path to config.json [default: config.json]");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for subcommand: no args or --help or unknown flags → print usage, exit 0.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1
        || args.iter().any(|a| a == "--help" || a == "-h")
        || (args.len() > 1 && args[1] != "serve")
    {
        print_usage();
        std::process::exit(0);
    }

    // ---------------------------------------------------------------------------
    // Initialise structured logging
    // ---------------------------------------------------------------------------
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "update_service=info,info".into()),
        )
        .init();

    // ---------------------------------------------------------------------------
    // Load configuration (REQ-7.1)
    // ---------------------------------------------------------------------------
    let config_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());

    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load configuration from '{}': {}", config_path, e);
            std::process::exit(1);
        }
    };

    // ---------------------------------------------------------------------------
    // Startup logging (REQ-8.1)
    // ---------------------------------------------------------------------------
    info!(
        version = env!("CARGO_PKG_VERSION"),
        port = config.grpc_port,
        registry_url = %config.registry_url,
        inactivity_timeout_secs = config.inactivity_timeout_secs,
        "update-service starting"
    );

    // ---------------------------------------------------------------------------
    // Build shared components
    // ---------------------------------------------------------------------------
    let manager: Arc<StateManager> = Arc::new(StateManager::new());

    let runtime: Arc<dyn ContainerRuntime> = Arc::new(PodmanRuntime {
        storage_path: config.container_storage_path.clone(),
    });

    // ---------------------------------------------------------------------------
    // Offload timer (REQ-6.1, REQ-6.3)
    // ---------------------------------------------------------------------------
    let offload_handle = spawn_offload_timer(
        Arc::clone(&manager),
        Arc::clone(&runtime),
        config.inactivity_timeout_secs,
    );

    // ---------------------------------------------------------------------------
    // gRPC server
    // ---------------------------------------------------------------------------
    let grpc_svc = UpdateServiceImpl::new(Arc::clone(&manager), Arc::clone(&runtime));

    let addr: SocketAddr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .expect("invalid gRPC bind address");

    info!("gRPC server listening on {}", addr);

    // Clones for the async shutdown closure
    let manager_shutdown = Arc::clone(&manager);
    let runtime_shutdown = Arc::clone(&runtime);

    tonic::transport::Server::builder()
        .add_service(UpdateServiceServer::new(grpc_svc))
        .serve_with_shutdown(addr, async move {
            // Block until a shutdown signal arrives
            shutdown_signal().await;

            info!("Shutdown signal received — stopping running adapters");

            // Stop any adapter that is currently RUNNING (REQ-8.2)
            if let Some(running_id) = manager_shutdown.get_running_adapter() {
                if let Some(info) = manager_shutdown.get(&running_id) {
                    let container_id = info
                        .container_id
                        .unwrap_or_else(|| running_id.clone());

                    if let Err(e) = runtime_shutdown.stop(&container_id).await {
                        warn!("Failed to stop container '{}' during shutdown: {}", container_id, e);
                    }

                    if let Err(e) =
                        manager_shutdown.transition(&running_id, AdapterState::Stopped)
                    {
                        warn!("Failed to record STOPPED state for '{}': {}", running_id, e);
                    }
                }
            }

            info!("update-service shutdown complete");
        })
        .await?;

    // Cancel the background offload timer now that the server has stopped
    offload_handle.abort();

    Ok(())
}

// ---------------------------------------------------------------------------
// Signal helpers
// ---------------------------------------------------------------------------

/// Resolves when either SIGINT (Ctrl-C) or SIGTERM is received.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c  => {},
        _ = sigterm => {},
    }
}
