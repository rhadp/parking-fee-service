//! PARKING_OPERATOR_ADAPTOR entry point.
//!
//! Starts the gRPC server on the configured address, initializing the
//! operator REST client and session manager.

use parking_operator_adaptor::config::Config;
use parking_operator_adaptor::grpc_service::ParkingAdaptorService;
use parking_operator_adaptor::operator_client::OperatorClient;
use parking_operator_adaptor::proto::adaptor::parking_adaptor_server::ParkingAdaptorServer;
use parking_operator_adaptor::session_manager::SessionManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();

    eprintln!(
        "parking-operator-adaptor starting on {} (operator: {})",
        config.grpc_addr, config.operator_url
    );

    let operator = OperatorClient::new(&config.operator_url);
    let session_mgr = SessionManager::new();
    let service = ParkingAdaptorService::new(operator, session_mgr);

    let addr = config.grpc_addr.parse()?;

    eprintln!("parking-operator-adaptor gRPC server listening on {}", addr);

    tonic::transport::Server::builder()
        .add_service(ParkingAdaptorServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
