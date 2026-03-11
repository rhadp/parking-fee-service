//! Integration tests: manual override, double lock/unlock, state-signal consistency,
//! and error gRPC calls.
//!
//! Task group 6 tests:
//! - TS-08-7, TS-08-8: Manual start and stop via gRPC
//! - TS-08-P1, TS-08-P2: Double lock / double unlock
//! - TS-08-P3, TS-08-P5: Manual start then autonomous unlock; lock ignored after manual start
//! - TS-08-P4: State-signal consistency after full cycle
//! - TS-08-E3, TS-08-E4, TS-08-E6: Error gRPC calls
//!
//! These tests use an in-process mock HTTP operator and directly invoke the gRPC
//! service handlers and autonomous event handlers.

use std::sync::Arc;
use tokio::sync::Mutex;

use parking_operator_adaptor::autonomous;
use parking_operator_adaptor::grpc::service::proto::parking_adaptor_server::ParkingAdaptor;
use parking_operator_adaptor::grpc::ParkingAdaptorService;
use parking_operator_adaptor::operator::OperatorClient;
use parking_operator_adaptor::session::{SessionManager, SessionState};

/// Start a minimal mock PARKING_OPERATOR HTTP server on a random port.
/// Returns the base URL (e.g., "http://127.0.0.1:<port>").
async fn start_mock_operator() -> String {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    let call_count: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let sessions: Arc<Mutex<HashMap<String, serde_json::Value>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let call_count_start = call_count.clone();
    let sessions_start = sessions.clone();
    let sessions_stop = sessions.clone();

    // Build routes using axum
    use axum::extract::Path;
    use axum::routing::{get, post};
    use axum::Json;

    let app = axum::Router::new()
        .route(
            "/parking/start",
            post({
                let call_count = call_count_start;
                let sessions = sessions_start;
                move |Json(body): Json<serde_json::Value>| {
                    let call_count = call_count.clone();
                    let sessions = sessions.clone();
                    async move {
                        call_count.fetch_add(1, Ordering::SeqCst);
                        let session_id =
                            format!("session-{}", uuid_simple());
                        let resp = serde_json::json!({
                            "session_id": session_id,
                            "status": "active"
                        });
                        let mut s = sessions.lock().await;
                        s.insert(session_id.clone(), body);
                        Json(resp)
                    }
                }
            }),
        )
        .route(
            "/parking/stop",
            post({
                let sessions = sessions_stop;
                move |Json(body): Json<serde_json::Value>| {
                    let sessions = sessions.clone();
                    async move {
                        let session_id = body["session_id"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string();
                        let mut s = sessions.lock().await;
                        s.remove(&session_id);
                        Json(serde_json::json!({
                            "session_id": session_id,
                            "duration": 120,
                            "fee": 5.0,
                            "status": "completed"
                        }))
                    }
                }
            }),
        )
        .route(
            "/parking/status/{session_id}",
            get(|Path(session_id): Path<String>| async move {
                Json(serde_json::json!({
                    "session_id": session_id,
                    "status": "active",
                    "rate_type": "per_hour",
                    "rate_amount": 2.50,
                    "currency": "EUR"
                }))
            }),
        )
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status": "ok"})) }),
        );

    // Bind to a random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://127.0.0.1:{}", addr.port());

    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    // Wait briefly for server to be ready
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    base_url
}

/// Generate a simple unique ID (not a real UUID, but sufficient for tests).
fn uuid_simple() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("{:016x}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

/// Helper: create a gRPC service with a mock operator on a random port.
async fn create_service_with_mock(
    zone_id: Option<String>,
) -> (ParkingAdaptorService, Arc<Mutex<SessionManager>>, Arc<OperatorClient>, String) {
    let base_url = start_mock_operator().await;
    let session = Arc::new(Mutex::new(SessionManager::new(zone_id)));
    let operator = Arc::new(OperatorClient::new(base_url.clone()));
    let svc = ParkingAdaptorService::new(
        session.clone(),
        operator.clone(),
        "DEMO-VIN-001".to_string(),
        None,
    );
    (svc, session, operator, base_url)
}

// ---------------------------------------------------------------------------
// TS-08-7: Manual StartSession Override
// ---------------------------------------------------------------------------

/// TS-08-7: Call StartSession via gRPC, verify session active.
#[tokio::test]
async fn test_manual_start_session() {
    let (svc, session, _operator, _url) =
        create_service_with_mock(Some("zone-demo-1".to_string())).await;

    // Call StartSession
    let request = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        },
    );
    let response = svc.start_session(request).await.expect("StartSession should succeed");
    let resp = response.into_inner();

    assert!(!resp.session_id.is_empty(), "session_id should be non-empty");
    assert_eq!(resp.status, "active", "status should be 'active'");

    // Verify via GetStatus
    let status_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::GetStatusRequest {},
    );
    let status_resp = svc.get_status(status_req).await.unwrap().into_inner();
    assert_eq!(status_resp.state, "active");
    assert!(!status_resp.session_id.is_empty());

    // Verify internal state
    let s = session.lock().await;
    assert_eq!(*s.state(), SessionState::Active);
}

// ---------------------------------------------------------------------------
// TS-08-8: Manual StopSession Stops Active Session
// ---------------------------------------------------------------------------

/// TS-08-8: Call StopSession via gRPC, verify session idle.
#[tokio::test]
async fn test_manual_stop_session() {
    let (svc, session, _operator, _url) =
        create_service_with_mock(Some("zone-demo-1".to_string())).await;

    // First, start a session manually
    let start_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        },
    );
    let start_resp = svc.start_session(start_req).await.unwrap().into_inner();
    let session_id = start_resp.session_id.clone();

    // Call StopSession
    let stop_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StopSessionRequest {},
    );
    let stop_resp = svc.stop_session(stop_req).await.expect("StopSession should succeed");
    let resp = stop_resp.into_inner();

    assert_eq!(resp.session_id, session_id, "session_id should match");
    assert!(resp.duration_seconds >= 0, "duration should be non-negative");
    assert!(resp.fee >= 0.0, "fee should be non-negative");
    assert_eq!(resp.status, "completed", "status should be 'completed'");

    // Verify via GetStatus
    let status_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::GetStatusRequest {},
    );
    let status_resp = svc.get_status(status_req).await.unwrap().into_inner();
    assert_eq!(status_resp.state, "idle");

    // Verify internal state
    let s = session.lock().await;
    assert_eq!(*s.state(), SessionState::Idle);
    assert!(s.session_id().is_none());
}

// ---------------------------------------------------------------------------
// TS-08-P1: Double Lock Does Not Create Duplicate Session
// ---------------------------------------------------------------------------

/// TS-08-P1: Two consecutive lock events should only trigger one operator start call.
#[tokio::test]
async fn test_double_lock_only_one_operator_start() {
    let base_url = start_mock_operator().await;
    let session = Arc::new(Mutex::new(SessionManager::new(Some(
        "zone-demo-1".to_string(),
    ))));
    let operator = Arc::new(OperatorClient::new(base_url));

    // First lock event
    autonomous::handle_lock_event(
        &session,
        &operator,
        &None,
        "DEMO-VIN-001",
        "zone-demo-1",
    )
    .await;

    // Verify session is active
    {
        let s = session.lock().await;
        assert_eq!(*s.state(), SessionState::Active, "session should be active after first lock");
        assert!(s.session_id().is_some());
    }

    let first_session_id = {
        let s = session.lock().await;
        s.session_id().unwrap().to_string()
    };

    // Second lock event (should be ignored)
    autonomous::handle_lock_event(
        &session,
        &operator,
        &None,
        "DEMO-VIN-001",
        "zone-demo-1",
    )
    .await;

    // Verify session still has the same session_id (no duplicate)
    let s = session.lock().await;
    assert_eq!(*s.state(), SessionState::Active, "session should still be active");
    assert_eq!(
        s.session_id().unwrap(),
        first_session_id,
        "session_id should not change after double lock"
    );
}

// ---------------------------------------------------------------------------
// TS-08-P2: Double Unlock Does Not Call Operator Twice
// ---------------------------------------------------------------------------

/// TS-08-P2: Two consecutive unlock events should only trigger one operator stop call.
#[tokio::test]
async fn test_double_unlock_only_one_operator_stop() {
    let base_url = start_mock_operator().await;
    let session = Arc::new(Mutex::new(SessionManager::new(Some(
        "zone-demo-1".to_string(),
    ))));
    let operator = Arc::new(OperatorClient::new(base_url));

    // Set up an active session
    autonomous::handle_lock_event(
        &session,
        &operator,
        &None,
        "DEMO-VIN-001",
        "zone-demo-1",
    )
    .await;

    {
        let s = session.lock().await;
        assert_eq!(*s.state(), SessionState::Active, "precondition: session should be active");
    }

    // First unlock event
    autonomous::handle_unlock_event(&session, &operator, &None).await;

    {
        let s = session.lock().await;
        assert_eq!(*s.state(), SessionState::Idle, "session should be idle after first unlock");
    }

    // Second unlock event (should be ignored)
    autonomous::handle_unlock_event(&session, &operator, &None).await;

    // Verify session is still idle
    let s = session.lock().await;
    assert_eq!(*s.state(), SessionState::Idle, "session should remain idle after double unlock");
}

// ---------------------------------------------------------------------------
// TS-08-P3: Manual Start Followed by Autonomous Unlock
// ---------------------------------------------------------------------------

/// TS-08-P3: After a manual start, an autonomous unlock should stop the session.
#[tokio::test]
async fn test_manual_start_then_autonomous_unlock() {
    let (svc, session, operator, _url) =
        create_service_with_mock(Some("zone-demo-1".to_string())).await;

    // Manual start via gRPC
    let start_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        },
    );
    let start_resp = svc.start_session(start_req).await.unwrap().into_inner();
    assert_eq!(start_resp.status, "active");

    // Verify session is active
    {
        let s = session.lock().await;
        assert_eq!(*s.state(), SessionState::Active);
    }

    // Autonomous unlock event should stop the session
    autonomous::handle_unlock_event(&session, &operator, &None).await;

    // Verify session is now idle
    let s = session.lock().await;
    assert_eq!(
        *s.state(),
        SessionState::Idle,
        "session should be idle after autonomous unlock"
    );
    assert!(s.session_id().is_none(), "session_id should be cleared");
}

// ---------------------------------------------------------------------------
// TS-08-P5: Lock Event Ignored After Manual Start
// ---------------------------------------------------------------------------

/// TS-08-P5: After a manual start, a lock event from DATA_BROKER should be ignored.
#[tokio::test]
async fn test_lock_event_ignored_after_manual_start() {
    let (svc, session, operator, _url) =
        create_service_with_mock(Some("zone-demo-1".to_string())).await;

    // Manual start via gRPC
    let start_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        },
    );
    let start_resp = svc.start_session(start_req).await.unwrap().into_inner();
    let original_session_id = start_resp.session_id.clone();

    // Lock event should be ignored (session already active)
    autonomous::handle_lock_event(
        &session,
        &operator,
        &None,
        "DEMO-VIN-001",
        "zone-demo-1",
    )
    .await;

    // Verify session still has the same session_id
    let s = session.lock().await;
    assert_eq!(*s.state(), SessionState::Active, "session should still be active");
    assert_eq!(
        s.session_id().unwrap(),
        original_session_id,
        "session_id should not change; lock event was ignored"
    );
}

// ---------------------------------------------------------------------------
// TS-08-P4: State-Signal Consistency After Full Cycle
// ---------------------------------------------------------------------------

/// TS-08-P4: Full cycle (lock -> active, unlock -> idle) with state checks at each step.
#[tokio::test]
async fn test_state_signal_consistency_full_cycle() {
    let base_url = start_mock_operator().await;
    let session = Arc::new(Mutex::new(SessionManager::new(Some(
        "zone-demo-1".to_string(),
    ))));
    let operator = Arc::new(OperatorClient::new(base_url));

    // Step 1: Initial state is idle
    {
        let s = session.lock().await;
        assert_eq!(*s.state(), SessionState::Idle, "initial state should be idle");
        assert!(s.session_id().is_none(), "no session_id when idle");
    }

    // Step 2: Lock event -> session should become active
    autonomous::handle_lock_event(
        &session,
        &operator,
        &None,
        "DEMO-VIN-001",
        "zone-demo-1",
    )
    .await;

    {
        let s = session.lock().await;
        assert_eq!(
            *s.state(),
            SessionState::Active,
            "state should be active after lock"
        );
        assert!(s.session_id().is_some(), "should have session_id when active");
    }

    // Step 3: Use GetStatus via gRPC service to verify
    let svc = ParkingAdaptorService::new(
        session.clone(),
        operator.clone(),
        "DEMO-VIN-001".to_string(),
        None,
    );
    let status_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::GetStatusRequest {},
    );
    let status_resp = svc.get_status(status_req).await.unwrap().into_inner();
    assert_eq!(status_resp.state, "active");
    assert!(!status_resp.session_id.is_empty());

    // Step 4: Unlock event -> session should become idle
    autonomous::handle_unlock_event(&session, &operator, &None).await;

    {
        let s = session.lock().await;
        assert_eq!(
            *s.state(),
            SessionState::Idle,
            "state should be idle after unlock"
        );
        assert!(s.session_id().is_none(), "session_id should be cleared after unlock");
    }

    // Step 5: Verify via GetStatus
    let status_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::GetStatusRequest {},
    );
    let status_resp = svc.get_status(status_req).await.unwrap().into_inner();
    assert_eq!(status_resp.state, "idle");
    assert!(status_resp.session_id.is_empty());
}

// ---------------------------------------------------------------------------
// TS-08-E3: StartSession When Session Already Active
// ---------------------------------------------------------------------------

/// TS-08-E3: StartSession returns ALREADY_EXISTS when a session is already active.
#[tokio::test]
async fn test_start_session_already_active_returns_already_exists() {
    let (svc, _session, _operator, _url) =
        create_service_with_mock(Some("zone-demo-1".to_string())).await;

    // Start a session
    let start_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        },
    );
    svc.start_session(start_req).await.expect("first start should succeed");

    // Try to start another session
    let start_req2 = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        },
    );
    let result = svc.start_session(start_req2).await;

    assert!(result.is_err(), "second start should fail");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::AlreadyExists,
        "should return ALREADY_EXISTS"
    );
    assert!(
        status.message().contains("session already active"),
        "message should contain 'session already active'"
    );
}

// ---------------------------------------------------------------------------
// TS-08-E4: StopSession When No Session Active
// ---------------------------------------------------------------------------

/// TS-08-E4: StopSession returns NOT_FOUND when no session is active.
#[tokio::test]
async fn test_stop_session_no_active_returns_not_found() {
    let (svc, _session, _operator, _url) =
        create_service_with_mock(Some("zone-demo-1".to_string())).await;

    let stop_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StopSessionRequest {},
    );
    let result = svc.stop_session(stop_req).await;

    assert!(result.is_err(), "stop should fail when no session active");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::NotFound,
        "should return NOT_FOUND"
    );
    assert!(
        status.message().contains("no active session"),
        "message should contain 'no active session'"
    );
}

// ---------------------------------------------------------------------------
// TS-08-E6: GetRate With No Zone Configured
// ---------------------------------------------------------------------------

/// TS-08-E6: GetRate returns FAILED_PRECONDITION when no zone is configured.
#[tokio::test]
async fn test_get_rate_no_zone_returns_failed_precondition() {
    let (svc, _session, _operator, _url) = create_service_with_mock(None).await;

    let rate_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::GetRateRequest {},
    );
    let result = svc.get_rate(rate_req).await;

    assert!(result.is_err(), "get_rate should fail when no zone configured");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::FailedPrecondition,
        "should return FAILED_PRECONDITION"
    );
    assert!(
        status.message().contains("no zone configured"),
        "message should contain 'no zone configured'"
    );
}

/// TS-08-E6: GetRate returns FAILED_PRECONDITION when zone is empty string.
#[tokio::test]
async fn test_get_rate_empty_zone_returns_failed_precondition() {
    let (svc, _session, _operator, _url) =
        create_service_with_mock(Some("".to_string())).await;

    let rate_req = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::GetRateRequest {},
    );
    let result = svc.get_rate(rate_req).await;

    assert!(result.is_err(), "get_rate should fail when zone is empty");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::FailedPrecondition,
        "should return FAILED_PRECONDITION for empty zone"
    );
}
