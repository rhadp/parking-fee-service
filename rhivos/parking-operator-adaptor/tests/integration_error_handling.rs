//! Integration test: operator unreachable error handling (TS-08-E1, TS-08-E2)
//!
//! Tests that the adaptor correctly handles an unreachable PARKING_OPERATOR.
//!
//! TS-08-E1: Operator unreachable on session start -- session remains idle.
//! TS-08-E2: Operator unreachable on session stop -- adaptor transitions to idle
//!           to avoid stuck state; SessionActive may remain stale.
//!
//! These tests use the operator REST client directly (unit-level integration)
//! and the gRPC service with a deliberately unreachable operator URL.
//!
//! Run with: `cargo test -p parking-operator-adaptor --features integration --test integration_error_handling`

#![cfg(feature = "integration")]

use std::sync::Arc;
use tokio::sync::Mutex;

/// TS-08-E1: When the operator is unreachable and a lock event triggers a
/// start attempt via the autonomous loop handler, the session remains idle.
///
/// This test simulates the autonomous start flow by invoking the handle_lock_event
/// function with an operator client pointed at an unreachable address.
#[tokio::test]
async fn test_operator_unreachable_on_start_session_remains_idle() {
    use parking_operator_adaptor::operator::OperatorClient;
    use parking_operator_adaptor::session::SessionManager;

    // Use an unreachable operator URL
    let operator = Arc::new(OperatorClient::new("http://127.0.0.1:19876".to_string()));
    let session = Arc::new(Mutex::new(SessionManager::new(Some(
        "zone-demo-1".to_string(),
    ))));

    // Simulate the autonomous lock event handler
    parking_operator_adaptor::autonomous::handle_lock_event(
        &session,
        &operator,
        &None, // no publisher
        "DEMO-VIN-001",
        "zone-demo-1",
    )
    .await;

    // Session should remain idle because operator was unreachable
    let s = session.lock().await;
    assert_eq!(
        *s.state(),
        parking_operator_adaptor::session::SessionState::Idle,
        "session should remain idle when operator is unreachable"
    );
    assert!(
        s.session_id().is_none(),
        "session_id should be None when start failed"
    );
}

/// TS-08-E1: When operator is unreachable, gRPC StartSession returns UNAVAILABLE.
#[tokio::test]
async fn test_grpc_start_session_returns_unavailable_when_operator_unreachable() {
    use parking_operator_adaptor::grpc::service::proto::parking_adaptor_server::ParkingAdaptor;
    use parking_operator_adaptor::grpc::ParkingAdaptorService;
    use parking_operator_adaptor::operator::OperatorClient;
    use parking_operator_adaptor::session::SessionManager;

    let session = Arc::new(Mutex::new(SessionManager::new(Some(
        "zone-demo-1".to_string(),
    ))));
    let operator = Arc::new(OperatorClient::new("http://127.0.0.1:19876".to_string()));
    let svc =
        ParkingAdaptorService::new(session.clone(), operator, "DEMO-VIN-001".to_string(), None);

    let request = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        },
    );
    let result = svc.start_session(request).await;

    assert!(result.is_err(), "start_session should fail");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::Unavailable,
        "should return UNAVAILABLE when operator is unreachable"
    );

    // Session should be back to idle (fail_start was called)
    let s = session.lock().await;
    assert_eq!(
        *s.state(),
        parking_operator_adaptor::session::SessionState::Idle,
        "session should be idle after failed start"
    );
}

/// TS-08-E2: When operator is unreachable during stop, the adaptor transitions
/// to idle to avoid a stuck state. The SessionActive signal may remain stale.
#[tokio::test]
async fn test_operator_unreachable_on_stop_session_transitions_to_idle() {
    use parking_operator_adaptor::operator::OperatorClient;
    use parking_operator_adaptor::session::SessionManager;

    // Set up an active session
    let session = Arc::new(Mutex::new(SessionManager::new(Some(
        "zone-demo-1".to_string(),
    ))));
    {
        let mut s = session.lock().await;
        s.try_start().unwrap();
        s.confirm_start("session-active-123".to_string());
        assert_eq!(
            *s.state(),
            parking_operator_adaptor::session::SessionState::Active
        );
    }

    // Use an unreachable operator URL
    let operator = Arc::new(OperatorClient::new("http://127.0.0.1:19876".to_string()));

    // Simulate the autonomous unlock event handler
    parking_operator_adaptor::autonomous::handle_unlock_event(
        &session,
        &operator,
        &None, // no publisher
    )
    .await;

    // Session should transition to idle to avoid stuck state,
    // even though the operator did not confirm the stop.
    let s = session.lock().await;
    assert_eq!(
        *s.state(),
        parking_operator_adaptor::session::SessionState::Idle,
        "session should transition to idle to avoid stuck state"
    );
}

/// TS-08-E2: When operator is unreachable during stop via gRPC, the service
/// returns an error and the session transitions to idle.
#[tokio::test]
async fn test_grpc_stop_session_returns_unavailable_when_operator_unreachable() {
    use parking_operator_adaptor::grpc::service::proto::parking_adaptor_server::ParkingAdaptor;
    use parking_operator_adaptor::grpc::ParkingAdaptorService;
    use parking_operator_adaptor::operator::OperatorClient;
    use parking_operator_adaptor::session::SessionManager;

    // Set up an active session
    let session = Arc::new(Mutex::new(SessionManager::new(Some(
        "zone-demo-1".to_string(),
    ))));
    {
        let mut s = session.lock().await;
        s.try_start().unwrap();
        s.confirm_start("session-active-456".to_string());
    }

    let operator = Arc::new(OperatorClient::new("http://127.0.0.1:19876".to_string()));
    let svc =
        ParkingAdaptorService::new(session.clone(), operator, "DEMO-VIN-001".to_string(), None);

    let request = tonic::Request::new(
        parking_operator_adaptor::grpc::service::proto::StopSessionRequest {},
    );
    let result = svc.stop_session(request).await;

    assert!(result.is_err(), "stop_session should fail");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::Unavailable,
        "should return UNAVAILABLE when operator is unreachable"
    );

    // Session should be back to idle (fail_stop transitions to idle)
    let s = session.lock().await;
    assert_eq!(
        *s.state(),
        parking_operator_adaptor::session::SessionState::Idle,
        "session should be idle after failed stop"
    );
}
