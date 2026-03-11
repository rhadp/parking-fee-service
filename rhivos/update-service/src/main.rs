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
use crate::container::ContainerRuntime;
use crate::grpc::UpdateServiceImpl;
use crate::manager::AdapterManager;
use crate::oci::{OciPuller, PodmanOciPuller};

/// Placeholder container runtime for initial server startup.
/// Will be replaced by PodmanRuntime in task group 4.
struct StubContainerRuntime;

#[async_trait::async_trait]
impl ContainerRuntime for StubContainerRuntime {
    async fn run(
        &self,
        _name: &str,
        _image_ref: &str,
    ) -> Result<(), container::ContainerError> {
        Err(container::ContainerError::RunFailed("not implemented".into()))
    }

    async fn stop(&self, _name: &str) -> Result<(), container::ContainerError> {
        Err(container::ContainerError::StopFailed("not implemented".into()))
    }

    async fn remove(&self, _name: &str) -> Result<(), container::ContainerError> {
        Err(container::ContainerError::RemoveFailed(
            "not implemented".into(),
        ))
    }

    async fn status(
        &self,
        _name: &str,
    ) -> Result<container::ContainerStatus, container::ContainerError> {
        Err(container::ContainerError::StatusFailed(
            "not implemented".into(),
        ))
    }
}

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

    // Create adapter manager with real OCI puller and stub container runtime
    let oci_puller: Arc<dyn OciPuller> = Arc::new(PodmanOciPuller::new());
    let container_runtime: Arc<dyn ContainerRuntime> = Arc::new(StubContainerRuntime);
    let manager = Arc::new(AdapterManager::new(oci_puller, container_runtime));

    let service = UpdateServiceImpl::new(manager);

    info!("UPDATE_SERVICE listening on {}", addr);

    Server::builder()
        .add_service(service.into_server())
        .serve(addr)
        .await?;

    Ok(())
}
