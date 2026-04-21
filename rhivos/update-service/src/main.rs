pub mod adapter;
pub mod config;
pub mod grpc_handler;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod proto;
pub mod service;
pub mod state;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tonic::transport::Server;
use tracing::info;

use config::load_config;
use grpc_handler::GrpcHandler;
use offload::spawn_offload_timer;
use podman::RealPodmanExecutor;
use proto::update::update_service_server::UpdateServiceServer;
use service::UpdateService as CoreService;
use state::StateManager;

#[tokio::main]
async fn main() {
    // Handle CLI: require "serve" subcommand.
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] != "serve" {
        if args.iter().skip(1).any(|a| a.starts_with('-')) {
            eprintln!("usage: update-service serve");
            std::process::exit(0);
        }
        println!("usage: update-service serve");
        std::process::exit(0);
    }

    // Initialise structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load configuration.
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());
    let cfg = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    info!(
        port = cfg.grpc_port,
        inactivity_timeout_secs = cfg.inactivity_timeout_secs,
        registry_url = %cfg.registry_url,
        "update-service starting"
    );

    // Build shared components.
    let (tx, _rx) = broadcast::channel::<adapter::AdapterStateEvent>(512);
    let state_mgr = Arc::new(StateManager::new(tx.clone()));
    let podman = Arc::new(RealPodmanExecutor);

    // Spawn offload timer (check every 60 seconds).
    let _offload_handle = spawn_offload_timer(
        Arc::clone(&state_mgr),
        Arc::clone(&podman),
        Duration::from_secs(cfg.inactivity_timeout_secs),
        60,
    );

    // Build gRPC service.
    let core = Arc::new(CoreService::new(
        Arc::clone(&state_mgr),
        Arc::clone(&podman),
        tx,
    ));
    let handler = GrpcHandler::new(core);
    let svc = UpdateServiceServer::new(handler);

    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.grpc_port)
        .parse()
        .expect("invalid gRPC address");

    info!(addr = %addr, "update-service ready");

    // Graceful shutdown on SIGTERM/SIGINT with a 10-second drain timeout.
    let shutdown_signal = async {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("failed to register SIGINT handler");
            tokio::select! {
                _ = sigterm.recv() => { info!("received SIGTERM"); }
                _ = sigint.recv() => { info!("received SIGINT"); }
            }
        }
        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl-c");
            info!("received ctrl-c");
        }
    };

    Server::builder()
        .add_service(svc)
        .serve_with_shutdown(addr, shutdown_signal)
        .await
        .expect("gRPC server error");

    // Allow in-flight RPCs up to 10 seconds to drain (tonic handles graceful
    // drain internally; we give it a moment then exit cleanly).
    tokio::time::sleep(Duration::from_secs(10)).await;

    info!("update-service stopped");
    std::process::exit(0);
}
