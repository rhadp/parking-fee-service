use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tonic::transport::Server;
use tracing::{error, info, warn};

use parking_operator_adaptor::broker::{BrokerPublisher, BrokerSubscriber};
use parking_operator_adaptor::config::Config;
use parking_operator_adaptor::grpc::service::pb::parking_adaptor_server::ParkingAdaptorServer;
use parking_operator_adaptor::grpc::ParkingAdaptorService;
use parking_operator_adaptor::operator::OperatorClient;
use parking_operator_adaptor::session::{SessionManager, SessionState};

/// Runs the autonomous event loop that subscribes to DATA_BROKER
/// lock/unlock events and drives session state transitions.
///
/// Requirements: 08-REQ-2.1, 08-REQ-3.1, 08-REQ-4.1
async fn run_autonomous_loop(
    session: Arc<Mutex<SessionManager>>,
    operator: Arc<OperatorClient>,
    publisher: Arc<Mutex<Option<BrokerPublisher>>>,
    subscriber: BrokerSubscriber,
    vehicle_id: String,
    zone_id: String,
) {
    info!("Starting autonomous event loop");

    let lock_stream = match subscriber.subscribe_lock_events().await {
        Ok(stream) => stream,
        Err(e) => {
            error!(error = %e, "Failed to subscribe to lock events; autonomous mode inactive");
            return;
        }
    };

    tokio::pin!(lock_stream);

    while let Some(is_locked) = lock_stream.next().await {
        info!(is_locked, "Received lock event from DATA_BROKER");

        if is_locked {
            handle_lock_event(&session, &operator, &publisher, &vehicle_id, &zone_id).await;
        } else {
            handle_unlock_event(&session, &operator, &publisher).await;
        }
    }

    warn!("DATA_BROKER subscription stream ended; autonomous mode inactive");
}

/// Handles a lock event (IsLocked = true): starts a parking session if idle.
///
/// Requirements: 08-REQ-3.1, 08-REQ-3.E1
async fn handle_lock_event(
    session: &Arc<Mutex<SessionManager>>,
    operator: &Arc<OperatorClient>,
    publisher: &Arc<Mutex<Option<BrokerPublisher>>>,
    vehicle_id: &str,
    zone_id: &str,
) {
    let mut session_guard = session.lock().await;

    // Ignore if not idle (double lock / session already active -- 08-REQ-3.E1)
    if *session_guard.state() != SessionState::Idle {
        info!(
            state = ?session_guard.state(),
            "Lock event ignored: session not idle"
        );
        return;
    }

    if let Err(e) = session_guard.try_start(zone_id) {
        warn!(?e, "Failed to transition to Starting on lock event");
        return;
    }

    // Call operator REST API
    match operator.start_session(vehicle_id, zone_id).await {
        Ok(resp) => {
            session_guard.confirm_start(&resp.session_id);
            info!(session_id = %resp.session_id, "Autonomous session started");

            // Publish SessionActive = true to DATA_BROKER (08-REQ-6.1)
            let mut pub_guard = publisher.lock().await;
            if let Some(ref mut pub_client) = *pub_guard {
                pub_client.set_session_active(true).await;
            }
        }
        Err(err) => {
            error!(?err, "Operator start_session failed on lock event");
            session_guard.fail_start();
        }
    }
}

/// Handles an unlock event (IsLocked = false): stops a parking session if active.
///
/// Requirements: 08-REQ-4.1, 08-REQ-4.E1
async fn handle_unlock_event(
    session: &Arc<Mutex<SessionManager>>,
    operator: &Arc<OperatorClient>,
    publisher: &Arc<Mutex<Option<BrokerPublisher>>>,
) {
    let mut session_guard = session.lock().await;

    // Ignore if not active (double unlock / no active session -- 08-REQ-4.E1)
    if *session_guard.state() != SessionState::Active {
        info!(
            state = ?session_guard.state(),
            "Unlock event ignored: no active session"
        );
        return;
    }

    let session_id = match session_guard.try_stop() {
        Ok(id) => id,
        Err(e) => {
            warn!(?e, "Failed to transition to Stopping on unlock event");
            return;
        }
    };

    // Call operator REST API
    match operator.stop_session(&session_id).await {
        Ok(resp) => {
            session_guard.confirm_stop();
            info!(
                session_id = %resp.session_id,
                duration = resp.duration,
                fee = resp.fee,
                "Autonomous session stopped"
            );

            // Publish SessionActive = false to DATA_BROKER (08-REQ-6.2)
            let mut pub_guard = publisher.lock().await;
            if let Some(ref mut pub_client) = *pub_guard {
                pub_client.set_session_active(false).await;
            }
        }
        Err(err) => {
            error!(?err, "Operator stop_session failed on unlock event");
            session_guard.fail_stop();
        }
    }
}

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

    // Attempt to connect to DATA_BROKER publisher (best-effort at startup)
    let publisher: Arc<Mutex<Option<BrokerPublisher>>> =
        match BrokerPublisher::connect(&config.data_broker_addr).await {
            Ok(pub_client) => {
                info!("DATA_BROKER publisher connected");
                Arc::new(Mutex::new(Some(pub_client)))
            }
            Err(e) => {
                warn!(error = %e, "DATA_BROKER publisher unavailable; session signals will not be published");
                Arc::new(Mutex::new(None))
            }
        };

    // Set up the gRPC service, sharing session state with autonomous loop
    let grpc_service = ParkingAdaptorService::new(
        session.clone(),
        operator.clone(),
        publisher.clone(),
        config.vehicle_id.clone(),
        config.zone_id.clone(),
    );

    let addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    tracing::info!(%addr, "gRPC server listening");

    // Spawn autonomous event loop in background
    let auto_session = session.clone();
    let auto_operator = operator.clone();
    let auto_publisher = publisher.clone();
    let auto_vehicle_id = config.vehicle_id.clone();
    let auto_zone_id = config.zone_id.clone();
    let subscriber = BrokerSubscriber::new(&config.data_broker_addr);

    tokio::spawn(async move {
        run_autonomous_loop(
            auto_session,
            auto_operator,
            auto_publisher,
            subscriber,
            auto_vehicle_id,
            auto_zone_id,
        )
        .await;
    });

    Server::builder()
        .add_service(ParkingAdaptorServer::new(grpc_service))
        .serve(addr)
        .await?;

    Ok(())
}
