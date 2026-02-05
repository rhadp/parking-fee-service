//! UPDATE_SERVICE - Container Lifecycle Management
//!
//! This service runs in the RHIVOS QM partition and handles
//! dynamic adapter installation and lifecycle management.
//!
//! Communication:
//! - Receives commands from PARKING_APP via gRPC/TLS
//! - Downloads adapters from REGISTRY via HTTPS/OCI

use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Starting UPDATE_SERVICE...");

    // TODO: Initialize gRPC server
    // TODO: Implement InstallAdapter, UninstallAdapter, ListAdapters, WatchAdapterStates RPCs

    info!("UPDATE_SERVICE started successfully");

    // Keep the service running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down UPDATE_SERVICE...");

    Ok(())
}
