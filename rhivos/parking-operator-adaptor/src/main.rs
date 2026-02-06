//! PARKING_OPERATOR_ADAPTOR - Dynamic Parking Session Manager
//!
//! This service runs in the RHIVOS QM partition and handles
//! parking session management with operator-specific logic.
//!
//! Communication:
//! - Receives commands from PARKING_APP via gRPC/TLS
//! - Publishes parking state to DATA_BROKER via gRPC/UDS
//! - Subscribes to lock signals from DATA_BROKER

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::signal;
use tokio::sync::watch;
use tonic::transport::Server;
use tracing::{error, info};

use parking_operator_adaptor::config::ServiceConfig;
use parking_operator_adaptor::location::LocationReader;
use parking_operator_adaptor::logging::{init_tracing, EventType, Logger};
use parking_operator_adaptor::manager::SessionManager;
use parking_operator_adaptor::operator::OperatorApiClient;
use parking_operator_adaptor::poller::StatusPoller;
use parking_operator_adaptor::proto::parking::parking_adaptor_server::ParkingAdaptorServer;
use parking_operator_adaptor::publisher::StatePublisher;
use parking_operator_adaptor::service::ParkingAdaptorImpl;
use parking_operator_adaptor::store::SessionStore;
use parking_operator_adaptor::subscriber::SignalSubscriber;
use parking_operator_adaptor::zone::ZoneLookupClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    init_tracing();

    let logger = Logger::new("main");
    logger.log(EventType::ServiceStartup, "Starting PARKING_OPERATOR_ADAPTOR");

    // Load configuration
    let config = ServiceConfig::from_env();
    info!("Configuration loaded: listen_addr={}", config.listen_addr);

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Initialize components
    let location_reader = LocationReader::new(config.data_broker_socket.clone());

    let zone_lookup_client = ZoneLookupClient::new(
        config.parking_fee_service_url.clone(),
        config.api_max_retries,
        config.api_base_delay_ms,
        config.api_timeout_ms,
    );

    let operator_client = OperatorApiClient::new(
        config.operator_base_url.clone(),
        config.vehicle_id.clone(),
        config.api_max_retries,
        config.api_base_delay_ms,
        config.api_max_delay_ms,
        config.api_timeout_ms,
    );

    let state_publisher = StatePublisher::new(config.data_broker_socket.clone());

    let session_store = SessionStore::new(PathBuf::from(&config.storage_path));

    // Create session manager
    let session_manager = Arc::new(SessionManager::new(
        location_reader.clone(),
        zone_lookup_client,
        operator_client.clone(),
        state_publisher,
        session_store,
    ));

    // Initialize session manager (recover persisted session)
    if let Err(e) = session_manager.init().await {
        error!("Failed to initialize session manager: {}", e);
    }

    // Create signal subscriber
    let signal_subscriber = Arc::new(SignalSubscriber::new(
        config.data_broker_socket.clone(),
        session_manager.clone(),
        config.reconnect_max_attempts,
        config.reconnect_base_delay_ms,
        config.reconnect_max_delay_ms,
    ));

    // Start signal subscription
    let _subscriber_handle = {
        let subscriber = signal_subscriber.clone();
        tokio::spawn(async move {
            if let Err(e) = subscriber.start().await {
                error!("Signal subscriber failed: {}", e);
            }
        })
    };

    // Create status poller
    let mut status_poller = StatusPoller::new(
        session_manager.clone(),
        operator_client,
        config.poll_interval_seconds * 1000, // Convert to ms
        shutdown_rx.clone(),
    );

    // Start status polling
    let poller_handle = tokio::spawn(async move {
        status_poller.run().await;
    });

    // Create gRPC service
    let grpc_service = ParkingAdaptorImpl::new(session_manager.clone(), location_reader);

    // Build gRPC server
    let addr: SocketAddr = config.listen_addr.parse()?;
    info!("gRPC server listening on {}", addr);

    logger.log(EventType::ServiceStartup, "All components initialized");

    // Start gRPC server with graceful shutdown
    let grpc_handle = tokio::spawn(async move {
        Server::builder()
            .add_service(ParkingAdaptorServer::new(grpc_service))
            .serve_with_shutdown(addr, async move {
                let mut shutdown = shutdown_rx;
                loop {
                    tokio::select! {
                        _ = shutdown.changed() => {
                            if *shutdown.borrow() {
                                info!("gRPC server shutdown signal received");
                                break;
                            }
                        }
                    }
                }
            })
            .await
    });

    info!("PARKING_OPERATOR_ADAPTOR started successfully");

    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("SIGINT received, initiating shutdown");
        }
        _ = async {
            #[cfg(unix)]
            {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
                sigterm.recv().await;
            }
            #[cfg(not(unix))]
            futures::future::pending::<()>().await
        } => {
            info!("SIGTERM received, initiating shutdown");
        }
    }

    logger.log(EventType::ServiceShutdown, "Shutting down PARKING_OPERATOR_ADAPTOR");

    // Signal shutdown to all components
    let _ = shutdown_tx.send(true);

    // Wait for components to shutdown
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let _ = poller_handle.await;
        let _ = grpc_handle.await;
    })
    .await
    .ok();

    info!("PARKING_OPERATOR_ADAPTOR shutdown complete");

    Ok(())
}
