//! UPDATE_SERVICE entry point.
//!
//! Starts the gRPC server on the configured address, initializing the
//! adapter manager with the configured offload timeout.
//!
//! Requirements covered:
//! - 04-REQ-4.1: gRPC service on configurable network address

use std::sync::Arc;

use tokio::sync::Mutex;
use update_service::adapter_manager::AdapterManager;
use update_service::config::Config;
use update_service::grpc_service::UpdateServiceGrpc;
use update_service::parking::update::v1::update_service_server::UpdateServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();

    eprintln!(
        "update-service starting on {} (registry: {}, offload_timeout: {:?})",
        config.grpc_addr, config.registry_url, config.offload_timeout
    );

    let manager = Arc::new(Mutex::new(AdapterManager::new(config.offload_timeout)));
    let service = UpdateServiceGrpc::new(manager);

    let addr = config.grpc_addr.parse()?;

    eprintln!("update-service gRPC server listening on {}", addr);

    tonic::transport::Server::builder()
        .add_service(UpdateServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
