//! LOCKING_SERVICE - ASIL-B Door Locking Service
//!
//! This service runs in the RHIVOS safety partition and handles
//! door lock/unlock commands with safety-critical guarantees.
//!
//! Communication:
//! - Receives commands from CLOUD_GATEWAY_CLIENT via gRPC/UDS
//! - Publishes door state to DATA_BROKER via gRPC/UDS

use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Starting LOCKING_SERVICE...");

    // TODO: Initialize gRPC server
    // TODO: Connect to DATA_BROKER
    // TODO: Implement Lock, Unlock, GetLockState RPCs

    info!("LOCKING_SERVICE started successfully");

    // Keep the service running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down LOCKING_SERVICE...");

    Ok(())
}
