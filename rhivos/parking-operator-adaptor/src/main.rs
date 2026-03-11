pub mod broker;
pub mod config;
pub mod grpc;
pub mod operator;
pub mod session;

use config::Config;
use grpc::service::proto::parking_adaptor_server::ParkingAdaptorServer;
use grpc::ParkingAdaptorService;
use operator::OperatorClient;
use session::SessionManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_env();
    info!(
        grpc_port = config.grpc_port,
        operator_url = %config.parking_operator_url,
        data_broker_addr = %config.data_broker_addr,
        vehicle_id = %config.vehicle_id,
        zone_id = %config.zone_id,
        "parking-operator-adaptor starting"
    );

    // Create shared session manager
    let session = Arc::new(Mutex::new(SessionManager::new(Some(
        config.zone_id.clone(),
    ))));

    // Create operator REST client
    let operator = Arc::new(OperatorClient::new(config.parking_operator_url.clone()));

    // Create gRPC service
    let service = ParkingAdaptorService::new(
        session.clone(),
        operator,
        config.vehicle_id.clone(),
    );

    // Start gRPC server
    let addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    info!(%addr, "gRPC server listening");

    tonic::transport::Server::builder()
        .add_service(ParkingAdaptorServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
