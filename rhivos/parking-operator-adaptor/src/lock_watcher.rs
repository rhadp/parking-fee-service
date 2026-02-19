//! Lock event watcher for automatic parking session management.
//!
//! Subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on the Kuksa
//! Databroker and starts/stops parking sessions via the PARKING_OPERATOR REST
//! API when lock state changes.
//!
//! # Requirements
//!
//! - 04-REQ-1.1: Subscribe to `IsLocked` on DATA_BROKER via gRPC streaming.
//! - 04-REQ-1.2: On lock → call `POST /parking/start`, update session state.
//! - 04-REQ-1.3: Write `SessionActive = true` to DATA_BROKER after start.
//! - 04-REQ-1.4: On unlock → call `POST /parking/stop`, complete session.
//! - 04-REQ-1.5: Write `SessionActive = false` to DATA_BROKER after stop.
//! - 04-REQ-1.E1: Log error on operator failure, do not set SessionActive.
//! - 04-REQ-1.E2: Ignore duplicate lock events (lock while already locked).
//! - 04-REQ-1.E3: Ignore unlock events when no session is active.

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::operator_client::OperatorClient;
use crate::session::{ParkingSession, RateType, SessionStatus};

use parking_proto::kuksa_client::KuksaClient;
use parking_proto::signals;

/// Shared session state accessible by both the lock watcher and gRPC server.
pub type SessionState = Arc<Mutex<Option<ParkingSession>>>;

/// Subscribe to `IsLocked` events and manage parking sessions.
///
/// Runs indefinitely, re-subscribing on stream errors with a short delay.
pub async fn watch_lock_events(
    kuksa: KuksaClient,
    operator: OperatorClient,
    session_state: SessionState,
    config: Config,
) {
    loop {
        match run_watcher(&kuksa, &operator, &session_state, &config).await {
            Ok(()) => {
                info!("lock watcher subscription stream ended, resubscribing");
            }
            Err(e) => {
                error!(error = %e, "lock watcher error, resubscribing in 2s");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

async fn run_watcher(
    kuksa: &KuksaClient,
    operator: &OperatorClient,
    session_state: &SessionState,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("subscribing to {}", signals::DOOR_IS_LOCKED);
    let mut stream = kuksa.subscribe_bool(signals::DOOR_IS_LOCKED).await?;
    info!("lock watcher subscription established");

    while let Some(result) = stream.next().await {
        match result {
            Ok(is_locked) => {
                handle_lock_event(is_locked, kuksa, operator, session_state, config).await;
            }
            Err(e) => {
                error!(error = %e, "error in lock event subscription");
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}

async fn handle_lock_event(
    is_locked: bool,
    kuksa: &KuksaClient,
    operator: &OperatorClient,
    session_state: &SessionState,
    config: &Config,
) {
    let mut state = session_state.lock().await;

    if is_locked {
        if state.as_ref().is_some_and(|s| s.is_active()) {
            info!("lock event but session already active, ignoring (04-REQ-1.E2)");
            return;
        }

        let now = unix_now();
        match operator
            .start_session(&config.vehicle_vin, &config.zone_id, now)
            .await
        {
            Ok(resp) => {
                info!(
                    session_id = %resp.session_id,
                    "parking session started with operator"
                );

                *state = Some(ParkingSession {
                    session_id: resp.session_id,
                    vehicle_id: config.vehicle_vin.clone(),
                    zone_id: config.zone_id.clone(),
                    start_time: now,
                    end_time: None,
                    rate_type: RateType::from_str_loose(&resp.rate.rate_type),
                    rate_amount: resp.rate.rate_amount,
                    currency: resp.rate.currency.clone(),
                    total_fee: None,
                    status: SessionStatus::Active,
                });

                if let Err(e) = kuksa
                    .set_bool(signals::PARKING_SESSION_ACTIVE, true)
                    .await
                {
                    warn!(error = %e, "failed to write SessionActive=true");
                }
            }
            Err(e) => {
                error!(error = %e, "failed to start session (04-REQ-1.E1)");
            }
        }
    } else {
        let session = match state.as_ref() {
            Some(s) if s.is_active() => s.clone(),
            _ => {
                info!("unlock event but no active session, ignoring (04-REQ-1.E3)");
                return;
            }
        };

        let now = unix_now();
        match operator.stop_session(&session.session_id, now).await {
            Ok(resp) => {
                info!(
                    session_id = %session.session_id,
                    total_fee = resp.total_fee,
                    duration_seconds = resp.duration_seconds,
                    "parking session stopped with operator"
                );

                if let Some(ref mut s) = *state {
                    s.complete(now, resp.total_fee, resp.duration_seconds);
                }

                if let Err(e) = kuksa
                    .set_bool(signals::PARKING_SESSION_ACTIVE, false)
                    .await
                {
                    warn!(error = %e, "failed to write SessionActive=false");
                }
            }
            Err(e) => {
                error!(error = %e, "failed to stop session with operator");
            }
        }
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_starts_as_none() {
        let state: SessionState = Arc::new(Mutex::new(None));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let guard = state.lock().await;
            assert!(guard.is_none());
        });
    }

    #[tokio::test]
    async fn duplicate_lock_with_active_session_is_noop() {
        let state: SessionState = Arc::new(Mutex::new(Some(ParkingSession {
            session_id: "sess-001".into(),
            vehicle_id: "VIN1".into(),
            zone_id: "zone-1".into(),
            start_time: 1_708_300_800,
            end_time: None,
            rate_type: RateType::PerMinute,
            rate_amount: 0.05,
            currency: "EUR".into(),
            total_fee: None,
            status: SessionStatus::Active,
        })));

        let guard = state.lock().await;
        assert!(
            guard.as_ref().is_some_and(|s| s.is_active()),
            "active session should cause lock event to be ignored"
        );
    }

    #[tokio::test]
    async fn unlock_without_active_session_is_noop() {
        let state: SessionState = Arc::new(Mutex::new(None));
        let guard = state.lock().await;
        let has_active = guard.as_ref().is_some_and(|s| s.is_active());
        assert!(
            !has_active,
            "no active session means unlock should be ignored"
        );
    }

    #[tokio::test]
    async fn unlock_with_completed_session_is_noop() {
        let state: SessionState = Arc::new(Mutex::new(Some(ParkingSession {
            session_id: "sess-001".into(),
            vehicle_id: "VIN1".into(),
            zone_id: "zone-1".into(),
            start_time: 1_708_300_800,
            end_time: Some(1_708_301_100),
            rate_type: RateType::PerMinute,
            rate_amount: 0.05,
            currency: "EUR".into(),
            total_fee: Some(0.25),
            status: SessionStatus::Completed,
        })));

        let guard = state.lock().await;
        let has_active = guard.as_ref().is_some_and(|s| s.is_active());
        assert!(
            !has_active,
            "completed session should not be treated as active"
        );
    }

    #[test]
    fn unix_now_returns_positive() {
        assert!(unix_now() > 0);
    }

    // ── Integration-style tests with mock Kuksa + mock operator ──────────
    //
    // These tests exercise `handle_lock_event` end-to-end with:
    // - A mock Kuksa VAL gRPC server that tracks `PublishValue` (set_bool) calls
    // - A wiremock HTTP server standing in for the PARKING_OPERATOR REST API
    //
    // **Property 1 (Event-Session Invariant):** Lock → start_session + SessionActive=true.
    // **Property 2 (Session Idempotency):** Duplicate events are ignored.
    // **Property 8 (SessionActive Signal Accuracy):** SessionActive matches session state.

    use parking_proto::kuksa::val::v2 as proto;
    use parking_proto::kuksa::val::v2::val_server::{Val, ValServer};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// A minimal mock Kuksa VAL gRPC server that records `PublishValue` calls.
    #[derive(Debug)]
    struct MockKuksaVal {
        /// Records all (signal_path, bool_value) pairs written via PublishValue.
        published: Arc<Mutex<Vec<(String, bool)>>>,
    }

    impl MockKuksaVal {
        fn new() -> Self {
            Self {
                published: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn published_clone(&self) -> Arc<Mutex<Vec<(String, bool)>>> {
            self.published.clone()
        }
    }

    #[tonic::async_trait]
    impl Val for MockKuksaVal {
        async fn get_value(
            &self,
            _req: tonic::Request<proto::GetValueRequest>,
        ) -> Result<tonic::Response<proto::GetValueResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("not used in tests"))
        }

        async fn get_values(
            &self,
            _req: tonic::Request<proto::GetValuesRequest>,
        ) -> Result<tonic::Response<proto::GetValuesResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("not used in tests"))
        }

        type SubscribeStream = tokio_stream::wrappers::ReceiverStream<
            Result<proto::SubscribeResponse, tonic::Status>,
        >;

        async fn subscribe(
            &self,
            _req: tonic::Request<proto::SubscribeRequest>,
        ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
            // Return an empty stream for tests that don't need it.
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(tonic::Response::new(
                tokio_stream::wrappers::ReceiverStream::new(rx),
            ))
        }

        type SubscribeByIdStream = tokio_stream::wrappers::ReceiverStream<
            Result<proto::SubscribeByIdResponse, tonic::Status>,
        >;

        async fn subscribe_by_id(
            &self,
            _req: tonic::Request<proto::SubscribeByIdRequest>,
        ) -> Result<tonic::Response<Self::SubscribeByIdStream>, tonic::Status> {
            Err(tonic::Status::unimplemented("not used in tests"))
        }

        async fn actuate(
            &self,
            _req: tonic::Request<proto::ActuateRequest>,
        ) -> Result<tonic::Response<proto::ActuateResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("not used in tests"))
        }

        async fn batch_actuate(
            &self,
            _req: tonic::Request<proto::BatchActuateRequest>,
        ) -> Result<tonic::Response<proto::BatchActuateResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("not used in tests"))
        }

        async fn list_metadata(
            &self,
            _req: tonic::Request<proto::ListMetadataRequest>,
        ) -> Result<tonic::Response<proto::ListMetadataResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("not used in tests"))
        }

        async fn publish_value(
            &self,
            req: tonic::Request<proto::PublishValueRequest>,
        ) -> Result<tonic::Response<proto::PublishValueResponse>, tonic::Status> {
            let inner = req.into_inner();
            if let Some(signal_id) = inner.signal_id {
                if let Some(proto::signal_id::Signal::Path(path)) = signal_id.signal {
                    if let Some(dp) = inner.data_point {
                        if let Some(val) = dp.value {
                            if let Some(proto::value::TypedValue::Bool(b)) = val.typed_value {
                                self.published.lock().await.push((path, b));
                            }
                        }
                    }
                }
            }
            Ok(tonic::Response::new(proto::PublishValueResponse {}))
        }

        type OpenProviderStreamStream = tokio_stream::wrappers::ReceiverStream<
            Result<proto::OpenProviderStreamResponse, tonic::Status>,
        >;

        async fn open_provider_stream(
            &self,
            _req: tonic::Request<tonic::Streaming<proto::OpenProviderStreamRequest>>,
        ) -> Result<tonic::Response<Self::OpenProviderStreamStream>, tonic::Status> {
            Err(tonic::Status::unimplemented("not used in tests"))
        }

        async fn get_server_info(
            &self,
            _req: tonic::Request<proto::GetServerInfoRequest>,
        ) -> Result<tonic::Response<proto::GetServerInfoResponse>, tonic::Status> {
            Ok(tonic::Response::new(proto::GetServerInfoResponse {
                name: "mock-kuksa".into(),
                version: "test".into(),
                commit_hash: "".into(),
            }))
        }
    }

    /// Start a mock Kuksa gRPC server on a random port.
    /// Returns (KuksaClient, published_values_handle).
    async fn start_mock_kuksa() -> (KuksaClient, Arc<Mutex<Vec<(String, bool)>>>) {
        let mock = MockKuksaVal::new();
        let published = mock.published_clone();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(ValServer::new(mock))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .unwrap();
        });

        // Small delay to let server start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = KuksaClient::connect(&format!("http://{}", addr))
            .await
            .expect("failed to connect to mock kuksa");

        (client, published)
    }

    fn make_config() -> Config {
        Config {
            listen_addr: "0.0.0.0:50054".into(),
            databroker_addr: "http://localhost:55555".into(),
            parking_operator_url: "http://placeholder".into(),
            zone_id: "zone-1".into(),
            vehicle_vin: "DEMO0000000000001".into(),
        }
    }

    fn operator_start_response() -> serde_json::Value {
        serde_json::json!({
            "session_id": "sess-001",
            "status": "active",
            "rate": {
                "zone_id": "zone-1",
                "rate_type": "per_minute",
                "rate_amount": 0.05,
                "currency": "EUR"
            }
        })
    }

    fn operator_stop_response() -> serde_json::Value {
        serde_json::json!({
            "session_id": "sess-001",
            "status": "completed",
            "total_fee": 0.25,
            "duration_seconds": 300,
            "currency": "EUR"
        })
    }

    /// **Property 1:** Lock event (true) with no active session starts a session
    /// and writes SessionActive=true to DATA_BROKER.
    #[tokio::test]
    async fn lock_event_starts_session_and_sets_signal() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(operator_start_response()))
            .expect(1)
            .mount(&mock_op)
            .await;

        let operator = OperatorClient::new(&mock_op.uri());
        let session_state: SessionState = Arc::new(Mutex::new(None));
        let config = Config {
            parking_operator_url: mock_op.uri(),
            ..make_config()
        };

        // Simulate a lock event (is_locked = true)
        handle_lock_event(true, &kuksa, &operator, &session_state, &config).await;

        // Verify session was created
        let state = session_state.lock().await;
        assert!(state.is_some(), "session should be created on lock");
        let session = state.as_ref().unwrap();
        assert_eq!(session.session_id, "sess-001");
        assert!(session.is_active());
        assert_eq!(session.vehicle_id, "DEMO0000000000001");
        assert_eq!(session.zone_id, "zone-1");
        drop(state);

        // Verify SessionActive=true was written to Kuksa
        let writes = published.lock().await;
        assert_eq!(writes.len(), 1, "expected exactly one publish_value call");
        assert_eq!(writes[0].0, signals::PARKING_SESSION_ACTIVE);
        assert!(writes[0].1, "SessionActive should be true");
    }

    /// **Property 1:** Unlock event (false) with active session stops the session
    /// and writes SessionActive=false to DATA_BROKER.
    #[tokio::test]
    async fn unlock_event_stops_session_and_clears_signal() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(operator_stop_response()))
            .expect(1)
            .mount(&mock_op)
            .await;

        let operator = OperatorClient::new(&mock_op.uri());
        // Pre-populate with an active session
        let session_state: SessionState = Arc::new(Mutex::new(Some(ParkingSession {
            session_id: "sess-001".into(),
            vehicle_id: "DEMO0000000000001".into(),
            zone_id: "zone-1".into(),
            start_time: 1_708_300_800,
            end_time: None,
            rate_type: RateType::PerMinute,
            rate_amount: 0.05,
            currency: "EUR".into(),
            total_fee: None,
            status: SessionStatus::Active,
        })));
        let config = Config {
            parking_operator_url: mock_op.uri(),
            ..make_config()
        };

        // Simulate an unlock event (is_locked = false)
        handle_lock_event(false, &kuksa, &operator, &session_state, &config).await;

        // Verify session was completed
        let state = session_state.lock().await;
        assert!(state.is_some());
        let session = state.as_ref().unwrap();
        assert!(!session.is_active(), "session should be completed");
        assert_eq!(session.status, SessionStatus::Completed);
        assert_eq!(session.total_fee, Some(0.25));
        drop(state);

        // Verify SessionActive=false was written to Kuksa
        let writes = published.lock().await;
        assert_eq!(writes.len(), 1, "expected exactly one publish_value call");
        assert_eq!(writes[0].0, signals::PARKING_SESSION_ACTIVE);
        assert!(!writes[0].1, "SessionActive should be false");
    }

    /// **Property 2 (04-REQ-1.E2):** Duplicate lock event with active session
    /// does NOT call the operator or write to Kuksa.
    #[tokio::test]
    async fn duplicate_lock_event_does_not_call_operator() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        // No operator mocks — if the operator is called, wiremock will report
        // an unexpected request and the test would fail due to .expect(0).
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(operator_start_response()))
            .expect(0)
            .mount(&mock_op)
            .await;

        let operator = OperatorClient::new(&mock_op.uri());
        let session_state: SessionState = Arc::new(Mutex::new(Some(ParkingSession {
            session_id: "sess-001".into(),
            vehicle_id: "DEMO0000000000001".into(),
            zone_id: "zone-1".into(),
            start_time: 1_708_300_800,
            end_time: None,
            rate_type: RateType::PerMinute,
            rate_amount: 0.05,
            currency: "EUR".into(),
            total_fee: None,
            status: SessionStatus::Active,
        })));
        let config = Config {
            parking_operator_url: mock_op.uri(),
            ..make_config()
        };

        // Simulate a duplicate lock event
        handle_lock_event(true, &kuksa, &operator, &session_state, &config).await;

        // Session should remain unchanged
        let state = session_state.lock().await;
        assert!(state.as_ref().unwrap().is_active());
        drop(state);

        // No writes to Kuksa
        let writes = published.lock().await;
        assert!(writes.is_empty(), "no Kuksa writes on duplicate lock event");
    }

    /// **Property 2 (04-REQ-1.E3):** Unlock event with no active session
    /// does NOT call the operator or write to Kuksa.
    #[tokio::test]
    async fn unlock_no_session_does_not_call_operator() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(operator_stop_response()))
            .expect(0)
            .mount(&mock_op)
            .await;

        let operator = OperatorClient::new(&mock_op.uri());
        let session_state: SessionState = Arc::new(Mutex::new(None));
        let config = Config {
            parking_operator_url: mock_op.uri(),
            ..make_config()
        };

        // Simulate an unlock event with no session
        handle_lock_event(false, &kuksa, &operator, &session_state, &config).await;

        // No writes to Kuksa
        let writes = published.lock().await;
        assert!(writes.is_empty(), "no Kuksa writes on unlock without session");
    }

    /// **04-REQ-1.E1:** Operator error on session start does NOT set SessionActive
    /// and does NOT create a session.
    #[tokio::test]
    async fn operator_error_on_start_does_not_set_signal() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        // Operator returns a 500 error
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .expect(1)
            .mount(&mock_op)
            .await;

        let operator = OperatorClient::new(&mock_op.uri());
        let session_state: SessionState = Arc::new(Mutex::new(None));
        let config = Config {
            parking_operator_url: mock_op.uri(),
            ..make_config()
        };

        // Simulate a lock event — operator will fail
        handle_lock_event(true, &kuksa, &operator, &session_state, &config).await;

        // No session should be created
        let state = session_state.lock().await;
        assert!(state.is_none(), "no session should be created when operator fails");
        drop(state);

        // SessionActive should NOT be written
        let writes = published.lock().await;
        assert!(
            writes.is_empty(),
            "no Kuksa writes when operator fails (04-REQ-1.E1)"
        );
    }

    /// **Property 1 + 8:** Full lock→unlock cycle verifies both signals are written
    /// correctly: SessionActive=true on lock, SessionActive=false on unlock.
    #[tokio::test]
    async fn full_lock_unlock_cycle_writes_correct_signals() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(operator_start_response()))
            .mount(&mock_op)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(operator_stop_response()))
            .mount(&mock_op)
            .await;

        let operator = OperatorClient::new(&mock_op.uri());
        let session_state: SessionState = Arc::new(Mutex::new(None));
        let config = Config {
            parking_operator_url: mock_op.uri(),
            ..make_config()
        };

        // Lock → start session
        handle_lock_event(true, &kuksa, &operator, &session_state, &config).await;

        // Unlock → stop session
        handle_lock_event(false, &kuksa, &operator, &session_state, &config).await;

        // Verify both Kuksa writes
        let writes = published.lock().await;
        assert_eq!(writes.len(), 2, "expected two publish_value calls");
        assert_eq!(writes[0].0, signals::PARKING_SESSION_ACTIVE);
        assert!(writes[0].1, "first write should be SessionActive=true");
        assert_eq!(writes[1].0, signals::PARKING_SESSION_ACTIVE);
        assert!(!writes[1].1, "second write should be SessionActive=false");
    }

    /// **Property 2:** After a full cycle, a second lock event should start a new
    /// session (the previous completed session is replaced).
    #[tokio::test]
    async fn second_lock_after_complete_starts_new_session() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(operator_start_response()))
            .mount(&mock_op)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(operator_stop_response()))
            .mount(&mock_op)
            .await;

        let operator = OperatorClient::new(&mock_op.uri());
        let session_state: SessionState = Arc::new(Mutex::new(None));
        let config = Config {
            parking_operator_url: mock_op.uri(),
            ..make_config()
        };

        // First cycle: lock → unlock
        handle_lock_event(true, &kuksa, &operator, &session_state, &config).await;
        handle_lock_event(false, &kuksa, &operator, &session_state, &config).await;

        // Session is now completed — a new lock should NOT be ignored
        // (completed sessions don't count as "active")
        handle_lock_event(true, &kuksa, &operator, &session_state, &config).await;

        // Verify a new session was started
        let state = session_state.lock().await;
        assert!(state.as_ref().unwrap().is_active(), "new session should be active");
        drop(state);

        // Verify 3 Kuksa writes: true, false, true
        let writes = published.lock().await;
        assert_eq!(writes.len(), 3);
        assert!(writes[0].1);   // lock → true
        assert!(!writes[1].1);  // unlock → false
        assert!(writes[2].1);   // lock again → true
    }
}
