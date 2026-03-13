use parking_operator_adaptor::autonomous;
use parking_operator_adaptor::broker::BrokerSessionPublisher;
use parking_operator_adaptor::config::Config;
use parking_operator_adaptor::grpc::service::proto::parking_adaptor_server::ParkingAdaptorServer;
use parking_operator_adaptor::grpc::ParkingAdaptorService;
use parking_operator_adaptor::operator::{OperatorApi, OperatorClient, RetryOperatorClient};
use parking_operator_adaptor::session::SessionManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// Crate version from Cargo.toml, used in startup logging (08-REQ-8.1).
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum number of DATA_BROKER connection attempts (08-REQ-6.E1).
const BROKER_MAX_ATTEMPTS: usize = 5;

/// Exponential backoff delays for DATA_BROKER connection retries (08-REQ-6.E1).
const BROKER_RETRY_DELAYS: [Duration; 5] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(16),
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_env();

    // 08-REQ-8.1: Log version, port, operator URL, DATA_BROKER address, vehicle ID
    info!(
        version = VERSION,
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

    // Create operator REST client with retry logic
    let operator: Arc<RetryOperatorClient<OperatorClient>> = Arc::new(RetryOperatorClient::new(
        OperatorClient::new(config.parking_operator_url.clone()),
    ));

    // 08-REQ-6.E1: Connect to DATA_BROKER with exponential backoff retry,
    // up to 5 attempts. Exit non-zero if unreachable.
    let publisher: Arc<dyn parking_operator_adaptor::broker::SessionPublisher> = {
        let mut result = None;
        for (attempt, delay) in BROKER_RETRY_DELAYS.iter().enumerate() {
            match BrokerSessionPublisher::connect(&config.data_broker_addr).await {
                Ok(p) => {
                    info!("DATA_BROKER publisher connected");
                    result = Some(p);
                    break;
                }
                Err(e) => {
                    warn!(
                        attempt = attempt + 1,
                        max = BROKER_MAX_ATTEMPTS,
                        error = %e,
                        "DATA_BROKER connection attempt failed"
                    );
                    if attempt < BROKER_MAX_ATTEMPTS - 1 {
                        tokio::time::sleep(*delay).await;
                    }
                }
            }
        }
        match result {
            Some(p) => Arc::new(p),
            None => {
                error!(
                    "DATA_BROKER unreachable after {} attempts; exiting",
                    BROKER_MAX_ATTEMPTS
                );
                std::process::exit(1);
            }
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
    let auto_publisher = publisher.clone();
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

    // Start gRPC server with graceful shutdown (08-REQ-8.2)
    let addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    info!(%addr, "gRPC server listening");
    info!("parking-operator-adaptor ready");

    tonic::transport::Server::builder()
        .add_service(ParkingAdaptorServer::new(service))
        .serve_with_shutdown(addr, async {
            shutdown_signal().await;
        })
        .await?;

    // 08-REQ-8.2: On shutdown, stop any active session with the PARKING_OPERATOR,
    // close the DATA_BROKER connection, and exit with code 0.
    let active_session_id = {
        let s = session.lock().await;
        if s.is_active() {
            s.session_id().map(|id| id.to_string())
        } else {
            None
        }
    };

    if let Some(session_id) = active_session_id {
        info!(session_id = %session_id, "stopping active session on shutdown");
        match operator.stop_session(&session_id).await {
            Ok(resp) => {
                info!(session_id = %resp.session_id, "session stopped during shutdown");
                let mut s = session.lock().await;
                s.confirm_stop();
                if let Err(e) = publisher.set_session_active(false).await {
                    warn!(error = %e, "failed to publish SessionActive=false during shutdown");
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to stop session during shutdown");
            }
        }
    }

    info!("parking-operator-adaptor shut down gracefully");
    Ok(())
}

/// Wait for SIGTERM or SIGINT (08-REQ-8.2).
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("received SIGINT, initiating shutdown");
        },
        _ = terminate => {
            info!("received SIGTERM, initiating shutdown");
        },
    }
}
