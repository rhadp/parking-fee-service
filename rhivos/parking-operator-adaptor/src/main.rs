//! PARKING_OPERATOR_ADAPTOR — binary entry point.
//!
//! Wires together all library modules:
//!
//! 1. Load configuration from environment variables (08-REQ-7.1, 08-REQ-7.2).
//! 2. Initialise structured logging.
//! 3. Log startup info: version, port, operator URL, DATA_BROKER addr, vehicle ID (08-REQ-8.1).
//! 4. Connect to DATA_BROKER with exponential-backoff retry; exit non-zero on failure
//!    (08-REQ-6.E1).
//! 5. Spawn the autonomous session loop (subscribes to lock/unlock events).
//! 6. Start the gRPC server on the configured port.
//! 7. On SIGTERM/SIGINT: stop any active session, shut down gRPC server, exit 0
//!    (08-REQ-8.2).

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{error, info, warn};

use parking_operator_adaptor::{
    autonomous::run_autonomous_loop,
    broker::{subscriber::BrokerSubscriber, SessionPublisher},
    config::load_config,
    grpc_server::ParkingAdaptorImpl,
    grpc_service::ParkingService,
    operator::{OperatorApi, OperatorClient, RetryOperatorClient},
    proto::parking_adaptor::parking_adaptor_server::ParkingAdaptorServer,
    session::SessionManager,
};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // -----------------------------------------------------------------------
    // Initialise structured logging
    // -----------------------------------------------------------------------
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "parking_operator_adaptor=info,info".into()),
        )
        .init();

    // -----------------------------------------------------------------------
    // Load configuration (08-REQ-7.1, 08-REQ-7.2)
    // -----------------------------------------------------------------------
    let config = load_config();

    // -----------------------------------------------------------------------
    // Startup logging (08-REQ-8.1)
    // -----------------------------------------------------------------------
    info!(
        version = env!("CARGO_PKG_VERSION"),
        port = config.grpc_port,
        operator_url = %config.parking_operator_url,
        data_broker_addr = %config.data_broker_addr,
        vehicle_id = %config.vehicle_id,
        zone_id = %config.zone_id,
        "parking-operator-adaptor starting"
    );

    // -----------------------------------------------------------------------
    // Connect to DATA_BROKER with exponential-backoff retry (08-REQ-6.E1).
    // Exit non-zero if all 5 attempts fail.
    // -----------------------------------------------------------------------
    let subscriber = match BrokerSubscriber::connect(&config.data_broker_addr).await {
        Ok(s) => {
            info!(addr = %config.data_broker_addr, "connected to DATA_BROKER");
            s
        }
        Err(e) => {
            error!(
                error = %e,
                addr = %config.data_broker_addr,
                "failed to connect to DATA_BROKER after retries; exiting"
            );
            std::process::exit(1);
        }
    };

    // Share the channel: publisher reuses the subscriber's channel.
    let publisher: Arc<dyn SessionPublisher> = Arc::new(subscriber.make_publisher());

    // -----------------------------------------------------------------------
    // Build shared components
    // -----------------------------------------------------------------------
    let session: Arc<Mutex<SessionManager>> = Arc::new(Mutex::new(SessionManager::new(Some(
        config.zone_id.clone(),
    ))));

    let http_client = OperatorClient::new(config.parking_operator_url.clone());
    let operator: Arc<dyn OperatorApi> = Arc::new(RetryOperatorClient::new(http_client));

    // -----------------------------------------------------------------------
    // Autonomous session loop (08-REQ-1.1, 08-REQ-2.1, 08-REQ-6.1)
    // Spawned as a background task; subscribes to lock/unlock events and
    // triggers session start/stop automatically.
    // -----------------------------------------------------------------------
    {
        let loop_session = Arc::clone(&session);
        let loop_operator = Arc::clone(&operator);
        let loop_publisher = Arc::clone(&publisher);
        let loop_broker_addr = config.data_broker_addr.clone();
        let loop_vehicle_id = config.vehicle_id.clone();
        let loop_zone_id = config.zone_id.clone();

        tokio::spawn(async move {
            run_autonomous_loop(
                loop_broker_addr,
                loop_session,
                loop_operator,
                loop_publisher,
                loop_vehicle_id,
                loop_zone_id,
            )
            .await;
        });
    }

    // -----------------------------------------------------------------------
    // Build and start the gRPC server
    // -----------------------------------------------------------------------
    let grpc_svc = ParkingService::new(
        Arc::clone(&session),
        Arc::clone(&operator),
        Arc::clone(&publisher),
        config.vehicle_id.clone(),
    );
    let adaptor_impl = ParkingAdaptorImpl::new(grpc_svc);

    let addr: SocketAddr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .expect("invalid gRPC bind address");

    info!(addr = %addr, "gRPC server listening");

    // Clones for the shutdown closure
    let session_shutdown = Arc::clone(&session);
    let operator_shutdown = Arc::clone(&operator);
    let publisher_shutdown = Arc::clone(&publisher);

    if let Err(e) = tonic::transport::Server::builder()
        .add_service(ParkingAdaptorServer::new(adaptor_impl))
        .serve_with_shutdown(addr, async move {
            // Wait for SIGTERM or SIGINT
            shutdown_signal().await;

            info!("shutdown signal received — stopping active session if any");

            // Stop active session before exiting (08-REQ-8.2).
            let session_id = {
                let s = session_shutdown.lock().await;
                if s.is_active() {
                    s.session_id().map(|id| id.to_string())
                } else {
                    None
                }
            };

            if let Some(sid) = session_id {
                info!(session_id = %sid, "stopping active session during shutdown");

                match operator_shutdown.stop_session(&sid).await {
                    Ok(_) => {
                        // Clear session state
                        {
                            let mut s = session_shutdown.lock().await;
                            let _ = s.stop();
                        }
                        // Publish SessionActive=false
                        if let Err(e) = publisher_shutdown.set_session_active(false).await {
                            warn!(error = %e, "failed to publish SessionActive=false during shutdown");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "failed to stop session during shutdown");
                    }
                }
            }

            info!("parking-operator-adaptor shutdown complete");
        })
        .await
    {
        error!(error = %e, "gRPC server error");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Signal helpers
// ---------------------------------------------------------------------------

/// Resolves when either SIGINT (Ctrl-C) or SIGTERM is received.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c  => {},
        _ = sigterm => {},
    }
}
