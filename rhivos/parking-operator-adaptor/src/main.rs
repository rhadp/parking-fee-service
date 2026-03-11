use parking_operator_adaptor::autonomous;
use parking_operator_adaptor::config::Config;
use parking_operator_adaptor::grpc::service::proto::parking_adaptor_server::ParkingAdaptorServer;
use parking_operator_adaptor::grpc::ParkingAdaptorService;
use parking_operator_adaptor::operator::OperatorClient;
use parking_operator_adaptor::session::SessionManager;
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

    // Create broker publisher (optional: only if DATA_BROKER is reachable)
    let publisher =
        match parking_operator_adaptor::broker::BrokerPublisher::connect(&config.data_broker_addr)
            .await
        {
            Ok(p) => {
                info!("DATA_BROKER publisher connected");
                Some(Arc::new(Mutex::new(p)))
            }
            Err(e) => {
                tracing::warn!(error = %e, "DATA_BROKER publisher not available at startup; session state signals will not be published");
                None
            }
        };

    // Create gRPC service
    let service = ParkingAdaptorService::new(
        session.clone(),
        operator.clone(),
        config.vehicle_id.clone(),
        publisher.clone(),
    );

    // Spawn autonomous event loop (DATA_BROKER subscription for lock/unlock events)
    let auto_session = session.clone();
    let auto_operator = operator.clone();
    let auto_publisher = publisher;
    let auto_vehicle_id = config.vehicle_id.clone();
    let auto_zone_id = config.zone_id.clone();
    let auto_broker_addr = config.data_broker_addr.clone();

    tokio::spawn(async move {
        autonomous::run_autonomous_loop(
            auto_broker_addr,
            auto_session,
            auto_operator,
            auto_publisher,
            auto_vehicle_id,
            auto_zone_id,
        )
        .await;
    });

    // Start gRPC server
    let addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    info!(%addr, "gRPC server listening");

    tonic::transport::Server::builder()
        .add_service(ParkingAdaptorServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
