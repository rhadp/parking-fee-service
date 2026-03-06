//! Integration tests for REST contract validation and error handling.
//!
//! Task group 5: Tests the PARKING_OPERATOR REST API contract directly
//! and validates error handling when the operator becomes unreachable.
//!
//! Test Spec Coverage:
//! - TS-08-9:  REST start/stop cycle (POST /parking/start, POST /parking/stop)
//! - TS-08-10: REST status query (GET /parking/status/{session_id})
//! - TS-08-E1: Operator unreachable on session start
//! - TS-08-E2: Operator unreachable on session stop

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use parking_operator_adaptor::operator::models::*;
use parking_operator_adaptor::operator::{OperatorClient, OperatorError};
use parking_operator_adaptor::session::{SessionManager, SessionState};
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// Mock operator infrastructure
// ---------------------------------------------------------------------------

/// Shared state for the mock parking operator.
#[derive(Clone)]
struct MockOperatorState {
    sessions: Arc<Mutex<HashMap<String, MockSession>>>,
    start_count: Arc<Mutex<u32>>,
    stop_count: Arc<Mutex<u32>>,
}

struct MockSession {
    vehicle_id: String,
    zone_id: String,
    start_timestamp: i64,
}

impl MockOperatorState {
    fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            start_count: Arc::new(Mutex::new(0)),
            stop_count: Arc::new(Mutex::new(0)),
        }
    }
}

fn mock_operator_app(state: MockOperatorState) -> Router {
    Router::new()
        .route("/parking/start", post(handle_start))
        .route("/parking/stop", post(handle_stop))
        .route("/parking/status/:session_id", get(handle_status))
        .with_state(state)
}

async fn handle_start(
    State(state): State<MockOperatorState>,
    Json(req): Json<StartRequest>,
) -> Json<serde_json::Value> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let mut sessions = state.sessions.lock().await;
    sessions.insert(
        session_id.clone(),
        MockSession {
            vehicle_id: req.vehicle_id,
            zone_id: req.zone_id,
            start_timestamp: req.timestamp,
        },
    );
    let mut count = state.start_count.lock().await;
    *count += 1;
    Json(serde_json::json!({
        "session_id": session_id,
        "status": "active"
    }))
}

async fn handle_stop(
    State(state): State<MockOperatorState>,
    Json(req): Json<StopRequest>,
) -> Json<serde_json::Value> {
    let sessions = state.sessions.lock().await;
    let duration = if let Some(session) = sessions.get(&req.session_id) {
        req.timestamp - session.start_timestamp
    } else {
        0
    };
    let mut count = state.stop_count.lock().await;
    *count += 1;
    Json(serde_json::json!({
        "session_id": req.session_id,
        "duration": duration,
        "fee": 2.50,
        "status": "completed"
    }))
}

async fn handle_status(
    State(state): State<MockOperatorState>,
    Path(session_id): Path<String>,
) -> Json<serde_json::Value> {
    let sessions = state.sessions.lock().await;
    let status = if sessions.contains_key(&session_id) {
        "active"
    } else {
        "unknown"
    };
    Json(serde_json::json!({
        "session_id": session_id,
        "status": status,
        "rate_type": "per_hour",
        "rate_amount": 2.50,
        "currency": "EUR"
    }))
}

/// Start the mock operator on a random port and return the address.
async fn start_mock_operator(state: MockOperatorState) -> SocketAddr {
    let app = mock_operator_app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

// ---------------------------------------------------------------------------
// 5.1 — REST start/stop cycle (TS-08-9, TS-08-10)
// ---------------------------------------------------------------------------

/// TS-08-9: Verify the complete REST start → stop cycle.
///
/// Sends POST /parking/start, records session_id, then sends
/// POST /parking/stop and validates all response fields.
#[tokio::test]
async fn test_rest_start_stop_cycle() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let client = OperatorClient::new(&format!("http://{addr}"));

    // --- POST /parking/start ---
    let start_resp = client.start_session("VIN-001", "zone-1").await.unwrap();
    assert!(!start_resp.session_id.is_empty(), "session_id must not be empty");
    assert_eq!(start_resp.status, "active", "start status must be 'active'");

    // Small delay to ensure a non-zero duration in the stop response.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // --- POST /parking/stop ---
    let stop_resp = client.stop_session(&start_resp.session_id).await.unwrap();
    assert_eq!(
        stop_resp.session_id, start_resp.session_id,
        "stop session_id must match start session_id"
    );
    assert!(stop_resp.duration >= 0, "duration must be non-negative");
    assert!(stop_resp.fee >= 0.0, "fee must be non-negative");
    assert_eq!(stop_resp.status, "completed", "stop status must be 'completed'");

    // Verify call counts
    let start_count = *state.start_count.lock().await;
    let stop_count = *state.stop_count.lock().await;
    assert_eq!(start_count, 1, "exactly one start call expected");
    assert_eq!(stop_count, 1, "exactly one stop call expected");
}

/// TS-08-9: Verify POST /parking/start request body contains required fields.
///
/// Validates that the operator client sends vehicle_id, zone_id, and
/// timestamp in the JSON body.
#[tokio::test]
async fn test_rest_start_request_fields() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let client = OperatorClient::new(&format!("http://{addr}"));

    let resp = client.start_session("VIN-001", "zone-1").await.unwrap();
    assert_eq!(resp.status, "active");

    // Verify the session was stored with correct fields
    let sessions = state.sessions.lock().await;
    let session = sessions.get(&resp.session_id).expect("session must exist");
    assert_eq!(session.vehicle_id, "VIN-001");
    assert_eq!(session.zone_id, "zone-1");
    assert!(session.start_timestamp > 0, "timestamp must be positive");
}

/// TS-08-10: Verify GET /parking/status/{session_id} returns status with rate info.
///
/// Starts a session, then queries its status and validates all response
/// fields including rate_type, rate_amount, and currency.
#[tokio::test]
async fn test_rest_status_query() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state).await;
    let client = OperatorClient::new(&format!("http://{addr}"));

    let start_resp = client.start_session("VIN-001", "zone-1").await.unwrap();
    let status_resp = client.get_status(&start_resp.session_id).await.unwrap();

    assert_eq!(status_resp.session_id, start_resp.session_id);
    assert_eq!(status_resp.status, "active");
    assert!(
        status_resp.rate_type == "per_hour" || status_resp.rate_type == "flat_fee",
        "rate_type must be 'per_hour' or 'flat_fee', got '{}'",
        status_resp.rate_type
    );
    assert!(status_resp.rate_amount > 0.0, "rate_amount must be positive");
    assert!(!status_resp.currency.is_empty(), "currency must not be empty");
}

/// TS-08-10: Verify GET /parking/status for an unknown session_id.
///
/// Queries status for a non-existent session; the mock returns "unknown".
#[tokio::test]
async fn test_rest_status_query_unknown_session() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state).await;
    let client = OperatorClient::new(&format!("http://{addr}"));

    let status_resp = client.get_status("non-existent-session").await.unwrap();
    assert_eq!(status_resp.session_id, "non-existent-session");
    assert_eq!(status_resp.status, "unknown");
}

// ---------------------------------------------------------------------------
// 5.2 — Operator unreachable (TS-08-E1, TS-08-E2)
// ---------------------------------------------------------------------------

/// TS-08-E1: Operator unreachable on session start — session remains idle.
///
/// When the operator is unreachable, a lock event should leave the session
/// in idle state. The state machine transitions to Starting and back to Idle
/// on failure.
#[tokio::test]
async fn test_operator_unreachable_on_start() {
    // No mock server started — operator is unreachable
    let client = OperatorClient::new("http://127.0.0.1:19996");
    let mut session = SessionManager::new();

    // Simulate lock event: idle -> starting
    assert_eq!(*session.state(), SessionState::Idle);
    session.try_start("zone-demo-1").unwrap();
    assert_eq!(*session.state(), SessionState::Starting);

    // Operator call fails
    let result = client.start_session("VIN-001", "zone-1").await;
    assert!(result.is_err(), "start_session must fail when operator unreachable");
    match result.unwrap_err() {
        OperatorError::Unreachable(_) => {} // expected
        other => panic!("expected Unreachable, got: {other:?}"),
    }

    // Fail start: starting -> idle
    session.fail_start();
    assert_eq!(
        *session.state(),
        SessionState::Idle,
        "session must return to idle after operator failure"
    );
    assert!(
        session.session_id().is_none(),
        "session_id must be None after failed start"
    );
}

/// TS-08-E2: Operator unreachable on session stop.
///
/// When the operator becomes unreachable after a session has been started,
/// a stop attempt should fail. The adaptor transitions to idle to avoid
/// a stuck state, but SessionActive may remain stale.
#[tokio::test]
async fn test_operator_unreachable_on_stop() {
    // Start a session with a reachable mock operator
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let reachable_client = OperatorClient::new(&format!("http://{addr}"));
    let mut session = SessionManager::new();

    // Start session successfully
    session.try_start("zone-demo-1").unwrap();
    let start_resp = reachable_client
        .start_session("VIN-001", "zone-1")
        .await
        .unwrap();
    session.confirm_start(&start_resp.session_id);
    assert_eq!(*session.state(), SessionState::Active);

    // Now simulate the operator becoming unreachable.
    // Use a client pointing to a port with nothing listening.
    let unreachable_client = OperatorClient::new("http://127.0.0.1:19995");

    // Simulate unlock event: active -> stopping
    let session_id = session.try_stop().unwrap();
    assert_eq!(*session.state(), SessionState::Stopping);
    assert_eq!(session_id, start_resp.session_id);

    // Operator call fails
    let result = unreachable_client.stop_session(&session_id).await;
    assert!(result.is_err(), "stop_session must fail when operator unreachable");
    match result.unwrap_err() {
        OperatorError::Unreachable(_) => {} // expected
        other => panic!("expected Unreachable, got: {other:?}"),
    }

    // Fail stop: stopping -> idle (to avoid stuck state)
    session.fail_stop();
    assert_eq!(
        *session.state(),
        SessionState::Idle,
        "session must transition to idle on stop failure to avoid stuck state"
    );

    // Verify the adaptor is functional and can start a new session
    session.try_start("zone-demo-1").unwrap();
    assert_eq!(*session.state(), SessionState::Starting);
}

/// TS-08-E1: Operator unreachable — session is recoverable after failure.
///
/// After an operator failure, the adaptor should accept new lock events
/// (session returns to idle and a new start attempt can succeed).
#[tokio::test]
async fn test_operator_failure_recovery() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let mut session = SessionManager::new();

    // First attempt: operator unreachable
    let bad_client = OperatorClient::new("http://127.0.0.1:19994");
    session.try_start("zone-demo-1").unwrap();
    let result = bad_client.start_session("VIN-001", "zone-1").await;
    assert!(result.is_err());
    session.fail_start();
    assert_eq!(*session.state(), SessionState::Idle);

    // Second attempt: operator now reachable
    let good_client = OperatorClient::new(&format!("http://{addr}"));
    session.try_start("zone-demo-1").unwrap();
    let resp = good_client.start_session("VIN-001", "zone-1").await.unwrap();
    session.confirm_start(&resp.session_id);
    assert_eq!(*session.state(), SessionState::Active);
    assert!(!resp.session_id.is_empty());

    let start_count = *state.start_count.lock().await;
    assert_eq!(start_count, 1, "only the successful call should reach the operator");
}

/// TS-08-E2: Operator unreachable on stop — full lifecycle with recovery.
///
/// After a stop failure due to operator being unreachable, the session
/// returns to idle and a new full lifecycle can complete successfully.
#[tokio::test]
async fn test_operator_stop_failure_then_new_lifecycle() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let client = OperatorClient::new(&format!("http://{addr}"));
    let mut session = SessionManager::new();

    // Start session successfully
    session.try_start("zone-demo-1").unwrap();
    let start_resp = client.start_session("VIN-001", "zone-1").await.unwrap();
    session.confirm_start(&start_resp.session_id);
    assert_eq!(*session.state(), SessionState::Active);

    // Unlock with unreachable operator
    let session_id = session.try_stop().unwrap();
    let bad_client = OperatorClient::new("http://127.0.0.1:19993");
    let result = bad_client.stop_session(&session_id).await;
    assert!(result.is_err());
    session.fail_stop();
    assert_eq!(*session.state(), SessionState::Idle);

    // New lifecycle succeeds
    session.try_start("zone-demo-1").unwrap();
    let start_resp2 = client.start_session("VIN-001", "zone-1").await.unwrap();
    session.confirm_start(&start_resp2.session_id);
    assert_eq!(*session.state(), SessionState::Active);

    let session_id2 = session.try_stop().unwrap();
    let stop_resp = client.stop_session(&session_id2).await.unwrap();
    session.confirm_stop();
    assert_eq!(*session.state(), SessionState::Idle);
    assert_eq!(stop_resp.status, "completed");

    let start_count = *state.start_count.lock().await;
    let stop_count = *state.stop_count.lock().await;
    assert_eq!(start_count, 2, "two successful start calls expected");
    assert_eq!(stop_count, 1, "one successful stop call expected");
}
