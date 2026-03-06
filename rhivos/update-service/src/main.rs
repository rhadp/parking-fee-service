pub mod config;
pub mod container;
pub mod grpc;
pub mod manager;
pub mod oci;
pub mod offload;
pub mod state;

/// Generated protobuf types for update_service.v1.
#[allow(clippy::doc_overindented_list_items)]
pub mod proto {
    tonic::include_proto!("update_service.v1");
}

use std::sync::Arc;

use tonic::transport::Server;
use tracing::info;

use crate::config::Config;
use crate::container::MockContainerRuntime;
use crate::grpc::UpdateServiceImpl;
use crate::manager::AdapterManager;
use crate::oci::MockOciPuller;
use crate::offload::OffloadTimer;
use crate::proto::update_service_server::UpdateServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::load(None).unwrap_or_else(|e| {
        tracing::warn!("Failed to load config, using defaults: {e}");
        Config::default()
    });

    info!("UPDATE_SERVICE starting on port {}", config.grpc_port);

    // Use mock implementations for now; real podman-backed impls come in later tasks.
    let oci: Arc<dyn crate::oci::OciPuller> = Arc::new(MockOciPuller::new());
    let container: Arc<dyn crate::container::ContainerRuntime> =
        Arc::new(MockContainerRuntime::new());

    let manager = Arc::new(AdapterManager::new(oci, container));

    // Start offload timer as a background task
    let offload_timer = OffloadTimer::new(
        Arc::clone(&manager),
        config.inactivity_timeout_secs,
    );
    tokio::spawn(async move {
        offload_timer.run().await;
    });

    let service = UpdateServiceImpl::new(manager);

    let addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    info!("Listening on {addr}");

    Server::builder()
        .add_service(UpdateServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
