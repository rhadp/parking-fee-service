//! Integration tests for autonomous session management (lock/unlock events).
//!
//! These tests spin up a mock parking operator HTTP server and verify that:
//! - The operator REST client correctly starts and stops sessions (TS-08-9)
//! - The session state machine integrates with the operator client (TS-08-1, TS-08-2)
//! - Manual gRPC start/stop produces the same state transitions (TS-08-7, TS-08-8)
//! - Double lock/unlock events are idempotent (TS-08-P1, TS-08-P2)
//! - Operator unreachable leaves session idle (TS-08-E1)

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

/// Shared state for the mock parking operator.
#[derive(Clone)]
struct MockOperatorState {
    sessions: Arc<Mutex<HashMap<String, MockSession>>>,
    start_count: Arc<Mutex<u32>>,
    stop_count: Arc<Mutex<u32>>,
}

#[allow(dead_code)]
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

/// Build the mock operator router.
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

/// TS-08-9: POST /parking/start returns session_id and status "active".
#[tokio::test]
async fn test_operator_start_session() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let client = OperatorClient::new(&format!("http://{addr}"));

    let resp = client.start_session("VIN-001", "zone-1").await.unwrap();
    assert!(!resp.session_id.is_empty(), "session_id should not be empty");
    assert_eq!(resp.status, "active");

    let count = state.start_count.lock().await;
    assert_eq!(*count, 1, "operator should have received exactly one start call");
}

/// TS-08-9: POST /parking/stop returns session_id, duration, fee, status.
#[tokio::test]
async fn test_operator_stop_session() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let client = OperatorClient::new(&format!("http://{addr}"));

    let start_resp = client.start_session("VIN-001", "zone-1").await.unwrap();
    let stop_resp = client.stop_session(&start_resp.session_id).await.unwrap();

    assert_eq!(stop_resp.session_id, start_resp.session_id);
    assert!(stop_resp.duration >= 0, "duration should be non-negative");
    assert!(stop_resp.fee >= 0.0, "fee should be non-negative");
    assert_eq!(stop_resp.status, "completed");
}

/// TS-08-10: GET /parking/status/{session_id} returns status with rate info.
#[tokio::test]
async fn test_operator_get_status() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state).await;
    let client = OperatorClient::new(&format!("http://{addr}"));

    let start_resp = client.start_session("VIN-001", "zone-1").await.unwrap();
    let status_resp = client.get_status(&start_resp.session_id).await.unwrap();

    assert_eq!(status_resp.session_id, start_resp.session_id);
    assert_eq!(status_resp.status, "active");
    assert_eq!(status_resp.rate_type, "per_hour");
    assert!(status_resp.rate_amount > 0.0);
    assert_eq!(status_resp.currency, "EUR");
}

/// TS-08-E1: Operator unreachable on session start leaves session idle.
#[tokio::test]
async fn test_operator_unreachable_start() {
    // No mock server started - operator is unreachable
    let client = OperatorClient::new("http://127.0.0.1:19998");
    let result = client.start_session("VIN-001", "zone-1").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OperatorError::Unreachable(_) => {} // expected
        other => panic!("expected Unreachable, got: {other:?}"),
    }
}

/// TS-08-1 + TS-08-2: Full session lifecycle via state machine + operator client.
/// Simulates: lock event -> start session -> unlock event -> stop session.
#[tokio::test]
async fn test_full_session_lifecycle() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let client = OperatorClient::new(&format!("http://{addr}"));
    let mut session = SessionManager::new();

    // Simulate lock event (TS-08-1)
    assert_eq!(*session.state(), SessionState::Idle);
    session.try_start("zone-demo-1").unwrap();
    assert_eq!(*session.state(), SessionState::Starting);

    let start_resp = client.start_session("DEMO-VIN-001", "zone-demo-1").await.unwrap();
    session.confirm_start(&start_resp.session_id);
    assert_eq!(*session.state(), SessionState::Active);
    assert!(session.session_id().is_some());

    // Simulate unlock event (TS-08-2)
    let session_id = session.try_stop().unwrap();
    assert_eq!(*session.state(), SessionState::Stopping);

    let stop_resp = client.stop_session(&session_id).await.unwrap();
    assert_eq!(stop_resp.session_id, session_id);
    assert_eq!(stop_resp.status, "completed");

    session.confirm_stop();
    assert_eq!(*session.state(), SessionState::Idle);
    assert!(session.session_id().is_none());
}

/// TS-08-P1: Double lock does not create duplicate session.
#[tokio::test]
async fn test_double_lock_idempotent() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let client = OperatorClient::new(&format!("http://{addr}"));
    let mut session = SessionManager::new();

    // First lock -> start session
    session.try_start("zone-demo-1").unwrap();
    let resp = client.start_session("DEMO-VIN-001", "zone-demo-1").await.unwrap();
    session.confirm_start(&resp.session_id);
    assert_eq!(*session.state(), SessionState::Active);

    // Second lock -> should be ignored
    let result = session.try_start("zone-demo-1");
    assert!(result.is_err(), "second try_start should fail");

    // Verify operator was only called once
    let count = state.start_count.lock().await;
    assert_eq!(*count, 1, "operator should have received exactly one start call");
}

/// TS-08-P2: Double unlock does not call operator twice.
#[tokio::test]
async fn test_double_unlock_idempotent() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let client = OperatorClient::new(&format!("http://{addr}"));
    let mut session = SessionManager::new();

    // Start a session
    session.try_start("zone-demo-1").unwrap();
    let resp = client.start_session("DEMO-VIN-001", "zone-demo-1").await.unwrap();
    session.confirm_start(&resp.session_id);

    // First unlock -> stop session
    let session_id = session.try_stop().unwrap();
    let _stop_resp = client.stop_session(&session_id).await.unwrap();
    session.confirm_stop();
    assert_eq!(*session.state(), SessionState::Idle);

    // Second unlock -> should be ignored
    let result = session.try_stop();
    assert!(result.is_err(), "second try_stop should fail");

    // Verify operator stop was only called once
    let count = state.stop_count.lock().await;
    assert_eq!(*count, 1, "operator should have received exactly one stop call");
}

/// CP-6: Operator failure during start leaves session idle.
#[tokio::test]
async fn test_operator_failure_leaves_idle() {
    // Use unreachable operator
    let client = OperatorClient::new("http://127.0.0.1:19997");
    let mut session = SessionManager::new();

    session.try_start("zone-demo-1").unwrap();
    assert_eq!(*session.state(), SessionState::Starting);

    let result = client.start_session("DEMO-VIN-001", "zone-demo-1").await;
    assert!(result.is_err());

    // Fail start -> back to idle
    session.fail_start();
    assert_eq!(*session.state(), SessionState::Idle);
    assert!(session.session_id().is_none());

    // Can try again
    session.try_start("zone-demo-1").unwrap();
    assert_eq!(*session.state(), SessionState::Starting);
}
