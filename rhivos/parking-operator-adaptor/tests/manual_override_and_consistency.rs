//! Integration tests for manual override and consistency (task group 6).
//!
//! These tests verify:
//! - TS-08-7:  Manual StartSession via gRPC starts a session
//! - TS-08-8:  Manual StopSession via gRPC stops an active session
//! - TS-08-P1: Double lock does not create duplicate session (integration level)
//! - TS-08-P2: Double unlock does not call operator twice (integration level)
//! - TS-08-P3: Manual start then autonomous unlock
//! - TS-08-P4: State-signal consistency after full cycle
//! - TS-08-P5: Lock event ignored after manual start
//! - TS-08-E3: StartSession when session already active returns ALREADY_EXISTS
//! - TS-08-E4: StopSession when no session active returns NOT_FOUND
//! - TS-08-E6: GetRate with no zone configured returns FAILED_PRECONDITION

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use parking_operator_adaptor::grpc::service::pb::parking_adaptor_server::ParkingAdaptor;
use parking_operator_adaptor::grpc::ParkingAdaptorService;
use parking_operator_adaptor::operator::models::*;
use parking_operator_adaptor::operator::OperatorClient;
use parking_operator_adaptor::session::SessionManager;
use tokio::sync::Mutex;
use tonic::Request;

// ---------------------------------------------------------------------------
// Mock operator infrastructure (shared across tests)
// ---------------------------------------------------------------------------

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

async fn start_mock_operator(state: MockOperatorState) -> SocketAddr {
    let app = mock_operator_app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

/// Helper: creates a ParkingAdaptorService backed by the given mock operator and
/// shared session manager.
fn make_service(
    session: Arc<Mutex<SessionManager>>,
    operator_url: &str,
    zone_id: &str,
) -> ParkingAdaptorService {
    let operator = Arc::new(OperatorClient::new(operator_url));
    let publisher = Arc::new(Mutex::new(None)); // no DATA_BROKER in unit-level integration
    ParkingAdaptorService::new(
        session,
        operator,
        publisher,
        "DEMO-VIN-001".to_string(),
        zone_id.to_string(),
    )
}

// ---------------------------------------------------------------------------
// 6.1 — Manual start and stop (TS-08-7, TS-08-8)
// ---------------------------------------------------------------------------

/// TS-08-7: Manual StartSession via gRPC starts a session.
///
/// Calls `StartSession` via the gRPC service trait, then verifies `GetStatus`
/// shows the session as active.
#[tokio::test]
async fn test_manual_start_session() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session, &format!("http://{addr}"), "zone-demo-1");

    // StartSession
    let resp = svc
        .start_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
                zone_id: "zone-demo-1".to_string(),
            },
        ))
        .await
        .expect("StartSession should succeed")
        .into_inner();

    assert!(!resp.session_id.is_empty(), "session_id must not be empty");
    assert_eq!(resp.status, "active");

    // GetStatus confirms active
    let status = svc
        .get_status(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.state, "active");
    assert_eq!(status.session_id, resp.session_id);

    // Verify operator received exactly one start call
    let count = *state.start_count.lock().await;
    assert_eq!(count, 1, "operator should receive exactly one start call");
}

/// TS-08-8: Manual StopSession via gRPC stops an active session.
///
/// Starts a session, then stops it via `StopSession` RPC, and verifies
/// the response fields and that `GetStatus` returns idle.
#[tokio::test]
async fn test_manual_stop_session() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session, &format!("http://{addr}"), "zone-demo-1");

    // Start first
    let start_resp = svc
        .start_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
                zone_id: "zone-demo-1".to_string(),
            },
        ))
        .await
        .unwrap()
        .into_inner();

    // StopSession
    let stop_resp = svc
        .stop_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StopSessionRequest {},
        ))
        .await
        .expect("StopSession should succeed")
        .into_inner();

    assert_eq!(stop_resp.session_id, start_resp.session_id);
    assert!(stop_resp.duration_seconds >= 0, "duration must be non-negative");
    assert!(stop_resp.fee >= 0.0, "fee must be non-negative");
    assert_eq!(stop_resp.status, "completed");

    // GetStatus confirms idle
    let status = svc
        .get_status(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.state, "idle");
    assert!(status.session_id.is_empty(), "session_id should be empty when idle");

    // Verify operator received one start and one stop
    assert_eq!(*state.start_count.lock().await, 1);
    assert_eq!(*state.stop_count.lock().await, 1);
}

// ---------------------------------------------------------------------------
// 6.2 — Double lock / double unlock (TS-08-P1, TS-08-P2) — integration level
// ---------------------------------------------------------------------------

/// TS-08-P1: Two consecutive lock events produce only one operator start call.
///
/// Uses the session manager + operator client integration to simulate
/// two lock events in sequence and verify idempotency.
#[tokio::test]
async fn test_double_lock_integration() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session, &format!("http://{addr}"), "zone-demo-1");

    // First lock -> start succeeds
    let resp1 = svc
        .start_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
                zone_id: "zone-demo-1".to_string(),
            },
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp1.status, "active");

    // Second lock -> should return ALREADY_EXISTS
    let result = svc
        .start_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
                zone_id: "zone-demo-1".to_string(),
            },
        ))
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::AlreadyExists);

    // Operator received exactly one start call
    assert_eq!(*state.start_count.lock().await, 1);

    // State is still active with the original session_id
    let status = svc
        .get_status(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.state, "active");
    assert_eq!(status.session_id, resp1.session_id);
}

/// TS-08-P2: Two consecutive unlock events produce only one operator stop call.
///
/// Starts a session, then issues two StopSession calls. The second should
/// return NOT_FOUND, and the operator should have received only one stop.
#[tokio::test]
async fn test_double_unlock_integration() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session, &format!("http://{addr}"), "zone-demo-1");

    // Start a session
    svc.start_session(Request::new(
        parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        },
    ))
    .await
    .unwrap();

    // First unlock -> stop succeeds
    let stop_resp = svc
        .stop_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StopSessionRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(stop_resp.status, "completed");

    // Second unlock -> NOT_FOUND
    let result = svc
        .stop_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StopSessionRequest {},
        ))
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);

    // Operator received exactly one stop call
    assert_eq!(*state.stop_count.lock().await, 1);

    // State is idle
    let status = svc
        .get_status(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.state, "idle");
}

// ---------------------------------------------------------------------------
// 6.3 — Manual start then autonomous unlock (TS-08-P3, TS-08-P5)
// ---------------------------------------------------------------------------

/// TS-08-P5: Lock event is ignored after manual start.
///
/// Manually starts a session via gRPC, then simulates a lock event
/// (try_start on the session manager). The lock event should be rejected
/// because the session is already active.
#[tokio::test]
async fn test_lock_event_ignored_after_manual_start() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session.clone(), &format!("http://{addr}"), "zone-demo-1");

    // Manual start via gRPC
    let start_resp = svc
        .start_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
                zone_id: "zone-demo-1".to_string(),
            },
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(start_resp.status, "active");

    // Simulate autonomous lock event: try_start should fail (session already active)
    {
        let mut s = session.lock().await;
        let result = s.try_start("zone-demo-1");
        assert!(result.is_err(), "lock event should be ignored when session is active");
    }

    // Operator received only one start (the manual one)
    assert_eq!(*state.start_count.lock().await, 1);

    // Session still has the same session_id
    let status = svc
        .get_status(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.state, "active");
    assert_eq!(status.session_id, start_resp.session_id);
}

/// TS-08-P3: Manual start followed by autonomous unlock.
///
/// Manually starts a session via gRPC, then stops it via the session
/// manager + operator client (simulating an autonomous unlock event).
/// The session should transition to idle.
#[tokio::test]
async fn test_manual_start_then_autonomous_unlock() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let operator = Arc::new(OperatorClient::new(&format!("http://{addr}")));
    let publisher = Arc::new(Mutex::new(None));
    let svc = ParkingAdaptorService::new(
        session.clone(),
        operator.clone(),
        publisher,
        "DEMO-VIN-001".to_string(),
        "zone-demo-1".to_string(),
    );

    // Manual start via gRPC
    let start_resp = svc
        .start_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
                zone_id: "zone-demo-1".to_string(),
            },
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(start_resp.status, "active");

    // Simulate autonomous unlock: session manager + operator
    {
        let mut s = session.lock().await;
        let session_id = s.try_stop().expect("try_stop should succeed from active state");
        assert_eq!(session_id, start_resp.session_id);

        let stop_resp = operator
            .stop_session(&session_id)
            .await
            .expect("operator stop should succeed");
        assert_eq!(stop_resp.status, "completed");
        s.confirm_stop();
    }

    // Verify idle via GetStatus
    let status = svc
        .get_status(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.state, "idle");
    assert!(status.session_id.is_empty());

    // Operator received one start (manual) and one stop (autonomous)
    assert_eq!(*state.start_count.lock().await, 1);
    assert_eq!(*state.stop_count.lock().await, 1);
}

// ---------------------------------------------------------------------------
// 6.4 — State-signal consistency (TS-08-P4)
// ---------------------------------------------------------------------------

/// TS-08-P4: State-signal consistency after full cycle.
///
/// Performs a full lock → active → unlock → idle cycle through the gRPC
/// service layer and verifies that at every step, GetStatus reflects the
/// correct internal state.
#[tokio::test]
async fn test_state_consistency_full_cycle() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session, &format!("http://{addr}"), "zone-demo-1");

    // Step 1: Initially idle
    let status = svc
        .get_status(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.state, "idle");
    assert!(status.session_id.is_empty());

    // Step 2: Start session -> active
    let start_resp = svc
        .start_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
                zone_id: "zone-demo-1".to_string(),
            },
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(start_resp.status, "active");

    // Step 3: Verify active state
    let status = svc
        .get_status(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.state, "active");
    assert_eq!(status.session_id, start_resp.session_id);
    assert_eq!(status.zone_id, "zone-demo-1");

    // Step 4: Stop session -> idle
    let stop_resp = svc
        .stop_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StopSessionRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(stop_resp.session_id, start_resp.session_id);
    assert_eq!(stop_resp.status, "completed");

    // Step 5: Verify idle state
    let status = svc
        .get_status(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.state, "idle");
    assert!(status.session_id.is_empty());

    // Operator call counts: exactly 1 start, 1 stop
    assert_eq!(*state.start_count.lock().await, 1);
    assert_eq!(*state.stop_count.lock().await, 1);
}

/// TS-08-P4 (extended): Multiple full cycles maintain consistency.
///
/// Runs two complete start→stop cycles and verifies state consistency
/// at every transition point.
#[tokio::test]
async fn test_state_consistency_multiple_cycles() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state.clone()).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session, &format!("http://{addr}"), "zone-demo-1");

    for cycle in 1..=2 {
        // Start
        let start_resp = svc
            .start_session(Request::new(
                parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
                    zone_id: "zone-demo-1".to_string(),
                },
            ))
            .await
            .unwrap_or_else(|e| panic!("cycle {cycle}: StartSession failed: {e}"))
            .into_inner();
        assert_eq!(start_resp.status, "active", "cycle {cycle}: start status");

        let status = svc
            .get_status(Request::new(
                parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
            ))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(status.state, "active", "cycle {cycle}: state after start");
        assert_eq!(
            status.session_id, start_resp.session_id,
            "cycle {cycle}: session_id after start"
        );

        // Stop
        let stop_resp = svc
            .stop_session(Request::new(
                parking_operator_adaptor::grpc::service::pb::StopSessionRequest {},
            ))
            .await
            .unwrap_or_else(|e| panic!("cycle {cycle}: StopSession failed: {e}"))
            .into_inner();
        assert_eq!(stop_resp.status, "completed", "cycle {cycle}: stop status");

        let status = svc
            .get_status(Request::new(
                parking_operator_adaptor::grpc::service::pb::GetStatusRequest {},
            ))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(status.state, "idle", "cycle {cycle}: state after stop");
        assert!(
            status.session_id.is_empty(),
            "cycle {cycle}: session_id after stop"
        );
    }

    assert_eq!(*state.start_count.lock().await, 2);
    assert_eq!(*state.stop_count.lock().await, 2);
}

// ---------------------------------------------------------------------------
// 6.5 — Error gRPC calls (TS-08-E3, TS-08-E4, TS-08-E6)
// ---------------------------------------------------------------------------

/// TS-08-E3: StartSession when session already active returns ALREADY_EXISTS.
///
/// Exercises the gRPC service layer to verify the correct error code.
#[tokio::test]
async fn test_grpc_start_session_already_active() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session, &format!("http://{addr}"), "zone-demo-1");

    // Start a session first
    svc.start_session(Request::new(
        parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        },
    ))
    .await
    .unwrap();

    // Second start -> ALREADY_EXISTS
    let result = svc
        .start_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StartSessionRequest {
                zone_id: "zone-demo-1".to_string(),
            },
        ))
        .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), tonic::Code::AlreadyExists);
    assert!(
        err.message().contains("session already active"),
        "error message should contain 'session already active', got: '{}'",
        err.message()
    );
}

/// TS-08-E4: StopSession when no session active returns NOT_FOUND.
///
/// Exercises the gRPC service layer to verify the correct error code.
#[tokio::test]
async fn test_grpc_stop_session_not_found() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session, &format!("http://{addr}"), "zone-demo-1");

    // Stop without any active session -> NOT_FOUND
    let result = svc
        .stop_session(Request::new(
            parking_operator_adaptor::grpc::service::pb::StopSessionRequest {},
        ))
        .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);
    assert!(
        err.message().contains("no active session"),
        "error message should contain 'no active session', got: '{}'",
        err.message()
    );
}

/// TS-08-E6: GetRate with no zone configured returns FAILED_PRECONDITION.
///
/// Creates a service with an empty zone_id and verifies the error code.
#[tokio::test]
async fn test_grpc_get_rate_no_zone() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    // Empty zone_id -> FAILED_PRECONDITION
    let svc = make_service(session, &format!("http://{addr}"), "");

    let result = svc
        .get_rate(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetRateRequest {},
        ))
        .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);
    assert!(
        err.message().contains("no zone configured"),
        "error message should contain 'no zone configured', got: '{}'",
        err.message()
    );
}

/// TS-08-E6 (inverse): GetRate with a valid zone returns rate information.
///
/// Sanity check that GetRate succeeds with a configured zone.
#[tokio::test]
async fn test_grpc_get_rate_success() {
    let state = MockOperatorState::new();
    let addr = start_mock_operator(state).await;
    let session = Arc::new(Mutex::new(SessionManager::new()));
    let svc = make_service(session, &format!("http://{addr}"), "zone-demo-1");

    let resp = svc
        .get_rate(Request::new(
            parking_operator_adaptor::grpc::service::pb::GetRateRequest {},
        ))
        .await
        .unwrap()
        .into_inner();

    assert!(
        resp.rate_type == "per_hour" || resp.rate_type == "flat_fee",
        "rate_type must be 'per_hour' or 'flat_fee', got: '{}'",
        resp.rate_type
    );
    assert!(resp.rate_amount > 0.0, "rate_amount must be positive");
    assert!(!resp.currency.is_empty(), "currency must not be empty");
    assert_eq!(resp.zone_id, "zone-demo-1");
}
