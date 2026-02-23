//! PARKING_OPERATOR_ADAPTOR entry point.
//!
//! Starts the gRPC server on the configured address, initializing the
//! operator REST client, session manager, DATA_BROKER client, and
//! autonomous event handler.

use std::sync::Arc;

use parking_operator_adaptor::config::Config;
use parking_operator_adaptor::databroker_client::KuksaDataBrokerClient;
use parking_operator_adaptor::event_handler::EventHandler;
use parking_operator_adaptor::grpc_service::ParkingAdaptorService;
use parking_operator_adaptor::operator_client::OperatorClient;
use parking_operator_adaptor::proto::adaptor::parking_adaptor_server::ParkingAdaptorServer;
use parking_operator_adaptor::session_manager::SessionManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();

    eprintln!(
        "parking-operator-adaptor starting on {} (operator: {}, databroker: {})",
        config.grpc_addr, config.operator_url, config.databroker_addr
    );

    let operator = OperatorClient::new(&config.operator_url);
    let session_mgr = SessionManager::new();

    // Create the DATA_BROKER client
    let databroker = Arc::new(KuksaDataBrokerClient::new(&config.databroker_addr));

    // Create the gRPC service with DATA_BROKER integration for overrides
    let service = ParkingAdaptorService::with_databroker(
        operator.clone(),
        session_mgr.clone(),
        databroker.clone(),
    );

    let addr = config.grpc_addr.parse()?;

    // Start DATA_BROKER connection with retry in the background
    let db_for_connect = databroker.clone();
    tokio::spawn(async move {
        db_for_connect.connect_with_retry().await;
    });

    // Start autonomous event handling in the background
    let event_handler = EventHandler::new(
        databroker,
        operator,
        session_mgr,
        config.vehicle_id.clone(),
    );
    tokio::spawn(async move {
        // Wait a bit for the DATA_BROKER connection to establish
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        event_handler.run().await;
    });

    eprintln!("parking-operator-adaptor gRPC server listening on {}", addr);

    tonic::transport::Server::builder()
        .add_service(ParkingAdaptorServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
