//! Update Service — manages adapter container lifecycle.
//!
//! This service manages adapter lifecycle operations (install, remove, status)
//! via gRPC. Container management uses podman; unused adapters are automatically
//! offloaded after a configurable timeout.
//!
//! # Requirements
//!
//! - 04-REQ-3.1: Create and start containers via podman.
//! - 04-REQ-3.6: Load persisted state and reconcile with podman.
//! - 04-REQ-4.1: Expose gRPC server implementing UpdateService.

pub mod config;
pub mod grpc_server;
pub mod offload;
pub mod podman;
pub mod state;

use std::sync::Arc;

use clap::Parser;
use tokio::signal;
use tokio::sync::Mutex;
use tracing::{error, info};

use parking_proto::services::update::update_service_server::UpdateServiceServer;

use crate::grpc_server::UpdateServiceImpl;
use crate::offload::OffloadManager;
use crate::podman::PodmanRunner;
use crate::state::AdapterConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cfg = config::Config::parse();

    let addr: std::net::SocketAddr = cfg.listen_addr.parse().map_err(|e| {
        error!("Invalid listen address '{}': {}", cfg.listen_addr, e);
        e
    })?;

    // Validate offload timeout at startup
    let offload_duration = cfg.offload_duration().map_err(|e| {
        error!("Invalid OFFLOAD_TIMEOUT '{}': {}", cfg.offload_timeout, e);
        e
    })?;

    info!(
        listen_addr = %addr,
        data_dir = %cfg.data_dir,
        offload_timeout = ?offload_duration,
        "update-service starting"
    );

    // Load persisted adapter state
    let store = state::AdapterStore::load(&cfg.data_dir).map_err(|e| {
        error!("Failed to load adapter state: {}", e);
        e
    })?;
    let store = Arc::new(Mutex::new(store));

    // Initialize components
    let runtime = Arc::new(PodmanRunner);
    let offload_mgr = OffloadManager::new(offload_duration);

    // Default adapter config — env vars passed to containers.
    // In production these would come from a config file or per-install request.
    let default_config = AdapterConfig::default();

    // Create the gRPC service
    let service = UpdateServiceImpl::new(
        store,
        runtime,
        offload_mgr.clone(),
        default_config,
    );

    // Reconcile persisted state with actual podman state
    service.reconcile().await;

    info!(listen_addr = %addr, "update-service gRPC server starting");

    tonic::transport::Server::builder()
        .add_service(UpdateServiceServer::new(service))
        .serve_with_shutdown(addr, async {
            signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl-c");
            info!("update-service shutting down");
            // Cancel all offload timers on shutdown
            offload_mgr.cancel_all().await;
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use config::Config;

    use super::*;

    #[test]
    fn cli_parses_default_args() {
        let cfg = Config::parse_from(["update-service"]);
        assert_eq!(cfg.listen_addr, "0.0.0.0:50053");
    }

    #[test]
    fn cli_parses_custom_listen_addr() {
        let cfg = Config::parse_from(["update-service", "--listen-addr", "127.0.0.1:9999"]);
        assert_eq!(cfg.listen_addr, "127.0.0.1:9999");
    }
}
