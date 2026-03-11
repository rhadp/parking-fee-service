pub mod broker;
pub mod config;
pub mod grpc;
pub mod operator;
pub mod session;

/// Autonomous session management module.
///
/// Subscribes to lock/unlock events from DATA_BROKER and manages parking
/// sessions automatically via the PARKING_OPERATOR REST API.
pub mod autonomous {
    use crate::broker::{BrokerPublisher, BrokerSubscriber};
    use crate::operator::OperatorClient;
    use crate::session::{SessionManager, SessionState};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tracing::{error, info, warn};

    /// Run the autonomous event loop that subscribes to lock/unlock events.
    ///
    /// This function connects to DATA_BROKER, subscribes to the IsLocked signal,
    /// and triggers session start/stop based on lock/unlock events.
    pub async fn run_autonomous_loop(
        broker_addr: String,
        session: Arc<Mutex<SessionManager>>,
        operator: Arc<OperatorClient>,
        publisher: Option<Arc<Mutex<BrokerPublisher>>>,
        vehicle_id: String,
        zone_id: String,
    ) {
        // Connect to DATA_BROKER with retry
        let mut subscriber = match BrokerSubscriber::connect(&broker_addr).await {
            Ok(s) => s,
            Err(e) => {
                error!(error = %e, "failed to connect BrokerSubscriber; autonomous mode disabled");
                return;
            }
        };

        // Subscribe to lock events
        let mut stream = match subscriber.subscribe_lock_events().await {
            Ok(s) => s,
            Err(e) => {
                error!(error = %e, "failed to subscribe to lock events; autonomous mode disabled");
                return;
            }
        };

        info!("autonomous event loop started; listening for lock/unlock events");

        // Process lock/unlock events
        use tokio_stream::StreamExt;
        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    if let Some(is_locked) = BrokerSubscriber::extract_is_locked(&response) {
                        if is_locked {
                            handle_lock_event(
                                &session,
                                &operator,
                                &publisher,
                                &vehicle_id,
                                &zone_id,
                            )
                            .await;
                        } else {
                            handle_unlock_event(&session, &operator, &publisher).await;
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "error receiving lock event from DATA_BROKER");
                }
            }
        }

        warn!("DATA_BROKER subscription stream ended; autonomous mode inactive");
    }

    /// Handle a lock event: start a parking session if idle.
    pub async fn handle_lock_event(
        session: &Arc<Mutex<SessionManager>>,
        operator: &Arc<OperatorClient>,
        publisher: &Option<Arc<Mutex<BrokerPublisher>>>,
        vehicle_id: &str,
        zone_id: &str,
    ) {
        let mut s = session.lock().await;

        // Only start if currently idle
        if *s.state() != SessionState::Idle {
            info!(state = ?s.state(), "lock event ignored; session not idle");
            return;
        }

        if let Err(e) = s.try_start() {
            info!(error = %e, "lock event ignored; cannot start session");
            return;
        }

        // Release lock before making the HTTP call
        drop(s);

        match operator.start_session(vehicle_id, zone_id).await {
            Ok(resp) => {
                info!(session_id = %resp.session_id, "autonomous session started");
                let mut s = session.lock().await;
                s.confirm_start(resp.session_id);

                // Publish SessionActive = true to DATA_BROKER
                if let Some(pub_ref) = publisher {
                    let mut pub_lock = pub_ref.lock().await;
                    if let Err(e) = pub_lock.set_session_active(true).await {
                        error!(error = %e, "failed to publish SessionActive=true");
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "autonomous start_session failed");
                let mut s = session.lock().await;
                s.fail_start();
            }
        }
    }

    /// Handle an unlock event: stop a parking session if active.
    pub async fn handle_unlock_event(
        session: &Arc<Mutex<SessionManager>>,
        operator: &Arc<OperatorClient>,
        publisher: &Option<Arc<Mutex<BrokerPublisher>>>,
    ) {
        let mut s = session.lock().await;

        // Only stop if currently active
        if *s.state() != SessionState::Active {
            info!(state = ?s.state(), "unlock event ignored; session not active");
            return;
        }

        if let Err(e) = s.try_stop() {
            info!(error = %e, "unlock event ignored; cannot stop session");
            return;
        }

        let session_id = s.session_id().unwrap_or_default().to_string();

        // Release lock before making the HTTP call
        drop(s);

        match operator.stop_session(&session_id).await {
            Ok(resp) => {
                info!(
                    session_id = %resp.session_id,
                    duration = resp.duration,
                    fee = resp.fee,
                    "autonomous session stopped"
                );
                let mut s = session.lock().await;
                s.confirm_stop();

                // Publish SessionActive = false to DATA_BROKER
                if let Some(pub_ref) = publisher {
                    let mut pub_lock = pub_ref.lock().await;
                    if let Err(e) = pub_lock.set_session_active(false).await {
                        error!(error = %e, "failed to publish SessionActive=false");
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "autonomous stop_session failed");
                let mut s = session.lock().await;
                s.fail_stop();
            }
        }
    }
}
