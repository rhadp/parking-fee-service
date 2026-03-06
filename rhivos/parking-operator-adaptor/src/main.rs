pub mod broker;
pub mod config;
pub mod grpc;
pub mod operator;
pub mod session;

/// Generated Kuksa VAL v2 protobuf types.
#[allow(clippy::doc_overindented_list_items)]
pub mod kuksa_proto {
    tonic::include_proto!("kuksa.val.v2");
}

use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;

use config::Config;
use grpc::service::pb::parking_adaptor_server::ParkingAdaptorServer;
use grpc::ParkingAdaptorService;
use operator::OperatorClient;
use session::SessionManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    tracing::info!(
        grpc_port = config.grpc_port,
        operator_url = %config.parking_operator_url,
        data_broker_addr = %config.data_broker_addr,
        vehicle_id = %config.vehicle_id,
        zone_id = %config.zone_id,
        "parking-operator-adaptor starting"
    );

    let session = Arc::new(Mutex::new(SessionManager::new()));
    let operator = Arc::new(OperatorClient::new(&config.parking_operator_url));

    let grpc_service = ParkingAdaptorService::new(
        session,
        operator,
        config.vehicle_id,
        config.zone_id,
    );

    let addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    tracing::info!(%addr, "gRPC server listening");

    Server::builder()
        .add_service(ParkingAdaptorServer::new(grpc_service))
        .serve(addr)
        .await?;

    Ok(())
}
