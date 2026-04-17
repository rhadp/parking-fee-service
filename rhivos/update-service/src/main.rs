/// UPDATE_SERVICE — gRPC server for containerized adapter lifecycle management.
///
/// Subcommands:
///   serve   Start the gRPC server (required — 01-REQ-4.1)
///
/// Environment:
///   CONFIG_PATH   Path to JSON config file (default: config.json)
use std::process;
use std::sync::Arc;
use std::time::Duration;

use tonic::transport::Server;
use tracing::info;

use update_service::config::{load_config, ConfigError};
use update_service::grpc_handler::UpdateServiceGrpc;
use update_service::offload::spawn_offload_timer;
use update_service::podman::RealPodmanExecutor;
use update_service::proto::update_service_server::UpdateServiceServer;
use update_service::service::UpdateService;
use update_service::state::StateManager;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // No subcommand or --help → print usage and exit 0 (01-REQ-4.1).
    if args.is_empty()
        || args
            .first()
            .map(|s| s == "--help" || s == "-h")
            .unwrap_or(false)
    {
        println!("update-service v{}", env!("CARGO_PKG_VERSION"));
        println!("Usage: update-service serve");
        return;
    }

    // Reject unknown flags (01-REQ-4.E1).
    for arg in &args {
        if arg.starts_with('-') {
            eprintln!("Unknown flag: {arg}");
            eprintln!("Usage: update-service serve");
            process::exit(1);
        }
    }

    match args[0].as_str() {
        "serve" => run_server().await,
        unknown => {
            eprintln!("Unknown subcommand: {unknown}");
            eprintln!("Usage: update-service serve");
            process::exit(1);
        }
    }
}

async fn run_server() {
    // Structured logging with optional RUST_LOG filter.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load configuration from CONFIG_PATH (default: config.json in cwd).
    let config_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());

    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(ConfigError::Json(e)) => {
            tracing::error!("Invalid JSON in config file {config_path}: {e}");
            process::exit(1);
        }
        Err(ConfigError::Io(e)) => {
            tracing::error!("Failed to read config file {config_path}: {e}");
            process::exit(1);
        }
    };

    // Startup log — includes port and inactivity timeout (07-REQ-10.1).
    info!(
        version = env!("CARGO_PKG_VERSION"),
        grpc_port = config.grpc_port,
        inactivity_timeout_secs = config.inactivity_timeout_secs,
        registry_url = %config.registry_url,
        "update-service starting"
    );

    let inactivity_timeout = Duration::from_secs(config.inactivity_timeout_secs);

    // Shared components.
    let (broadcaster, _broadcast_rx) = tokio::sync::broadcast::channel::<
        update_service::adapter::AdapterStateEvent,
    >(1000);
    let state = Arc::new(StateManager::new(broadcaster.clone()));
    let podman = Arc::new(RealPodmanExecutor);

    let svc = Arc::new(UpdateService::new(
        Arc::clone(&state),
        Arc::clone(&podman),
        broadcaster,
        inactivity_timeout,
    ));

    // Offload timer: poll every 60 seconds for expired STOPPED adapters.
    let offload_handle = spawn_offload_timer(
        Arc::clone(&state),
        Arc::clone(&podman),
        inactivity_timeout,
        Duration::from_secs(60),
    )
    .await;

    let grpc_service = UpdateServiceGrpc::new(Arc::clone(&svc));
    let addr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .expect("invalid bind address");

    // Oneshot channel for shutdown coordination.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Signal handler task: waits for SIGTERM/SIGINT, initiates graceful shutdown,
    // then force-terminates after 10 s if drain has not completed (07-REQ-10.E1).
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        info!("Shutdown signal received, draining in-flight RPCs (max 10 s)...");
        let _ = shutdown_tx.send(());

        // Hard limit: force-exit after 10 s regardless of drain state.
        tokio::time::sleep(Duration::from_secs(10)).await;
        info!("Force-terminating after 10 s drain timeout");
        process::exit(0);
    });

    // Start gRPC server; it will stop accepting new RPCs once shutdown_rx fires.
    let server = Server::builder()
        .add_service(UpdateServiceServer::new(grpc_service))
        .serve_with_shutdown(addr, async move {
            shutdown_rx.await.ok();
        });

    info!(port = config.grpc_port, "update-service ready");

    if let Err(e) = server.await {
        tracing::error!("gRPC server error: {e}");
        offload_handle.abort();
        process::exit(1);
    }

    offload_handle.abort();
    info!("update-service stopped");
}

/// Wait for SIGTERM (Unix) or SIGINT (all platforms).
async fn wait_for_shutdown_signal() {
    use tokio::signal;

    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM");
        tokio::select! {
            _ = signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
    }
}
