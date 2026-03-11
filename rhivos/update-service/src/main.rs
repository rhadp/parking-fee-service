pub mod config;
pub mod container;
pub mod grpc;
pub mod manager;
pub mod oci;
pub mod offload;
pub mod state;

use std::sync::Arc;

use tonic::transport::Server;
use tracing::{error, info};

use crate::config::Config;
use crate::container::{ContainerRuntime, PodmanRuntime};
use crate::grpc::UpdateServiceImpl;
use crate::manager::AdapterManager;
use crate::oci::{OciPuller, PodmanOciPuller};
use crate::offload::OffloadTimer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Load configuration
    let config_path = std::env::args().nth(1);
    let config = Config::load(config_path.as_deref()).unwrap_or_else(|e| {
        error!("Failed to load config: {}, using defaults", e);
        Config::default()
    });

    info!(
        port = config.grpc_port,
        registry = %config.registry_base_url,
        inactivity_timeout = config.inactivity_timeout_secs,
        storage = %config.storage_path,
        "UPDATE_SERVICE starting"
    );

    let addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;

    // Create adapter manager with real OCI puller and podman container runtime
    let oci_puller: Arc<dyn OciPuller> = Arc::new(PodmanOciPuller::new());
    let container_runtime: Arc<dyn ContainerRuntime> = Arc::new(PodmanRuntime::new());
    let manager = Arc::new(AdapterManager::new(oci_puller, container_runtime));

    let service = UpdateServiceImpl::new(manager.clone());

    // Start the offload timer as a background task
    let offload_timer = OffloadTimer::new(manager, config.inactivity_timeout_secs);
    tokio::spawn(async move {
        offload_timer.run().await;
    });

    info!("UPDATE_SERVICE listening on {}", addr);

    Server::builder()
        .add_service(service.into_server())
        .serve(addr)
        .await?;

    Ok(())
}
