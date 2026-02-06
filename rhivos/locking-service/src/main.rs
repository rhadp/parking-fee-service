//! LOCKING_SERVICE - ASIL-B Door Locking Service Binary
//!
//! This binary runs the LOCKING_SERVICE in the RHIVOS safety partition,
//! handling door lock/unlock commands with safety-critical guarantees.
//!
//! Communication:
//! - Receives commands from CLOUD_GATEWAY_CLIENT via gRPC/UDS
//! - Publishes door state to DATA_BROKER via gRPC/UDS

use std::sync::Arc;

use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::info;

use locking_service::config::ServiceConfig;
use locking_service::error::LockingError;
use locking_service::proto::locking_service_server::LockingServiceServer;
use locking_service::service::LockingServiceImpl;
use locking_service::state::LockState;

/// Placeholder signal reader that connects to DATA_BROKER.
/// In production, this would use a real gRPC client.
#[derive(Clone)]
struct DataBrokerSignalReader;

#[async_trait::async_trait]
impl locking_service::validator::SignalReader for DataBrokerSignalReader {
    async fn read_bool(&self, path: &str) -> Result<bool, LockingError> {
        // TODO: Implement actual DATA_BROKER connection
        // For now, return safe defaults
        tracing::debug!("Reading bool signal: {}", path);
        // Doors closed by default, other bool signals also default to false
        let _ = path; // Acknowledge the path even though we return default
        Ok(false)
    }

    async fn read_float(&self, path: &str) -> Result<f32, LockingError> {
        // TODO: Implement actual DATA_BROKER connection
        tracing::debug!("Reading float signal: {}", path);
        // Vehicle stationary by default, other float signals also default to 0.0
        let _ = path; // Acknowledge the path even though we return default
        Ok(0.0)
    }
}

/// Placeholder signal writer that publishes to DATA_BROKER.
/// In production, this would use a real gRPC client.
#[derive(Clone)]
struct DataBrokerSignalWriter;

#[async_trait::async_trait]
impl locking_service::publisher::SignalWriter for DataBrokerSignalWriter {
    async fn write_bool(&self, path: &str, value: bool) -> Result<(), LockingError> {
        // TODO: Implement actual DATA_BROKER connection
        tracing::debug!("Writing bool signal: {} = {}", path, value);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("locking_service=info".parse()?),
        )
        .init();

    info!("Starting LOCKING_SERVICE...");

    // Load configuration
    let config = ServiceConfig::from_env();
    info!(
        socket_path = %config.socket_path,
        data_broker_socket = %config.data_broker_socket,
        "Configuration loaded"
    );

    // Initialize shared state
    let lock_state = Arc::new(RwLock::new(LockState::default()));

    // Create signal reader/writer (placeholder implementations)
    let signal_reader = DataBrokerSignalReader;
    let signal_writer = DataBrokerSignalWriter;

    // Create the service implementation
    let service = LockingServiceImpl::new(
        Arc::clone(&lock_state),
        signal_reader,
        signal_writer,
        config.clone(),
    );

    // Create gRPC server
    let addr = "0.0.0.0:50051".to_string(); // TCP for development, UDS for production
    info!(addr = %addr, "Starting gRPC server");

    // Spawn graceful shutdown handler
    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        info!("Received shutdown signal");
    };

    // Start server with graceful shutdown
    Server::builder()
        .add_service(LockingServiceServer::new(service))
        .serve_with_shutdown(addr.parse()?, shutdown)
        .await?;

    info!("LOCKING_SERVICE shut down gracefully");
    Ok(())
}
