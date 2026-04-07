use crate::broker::DataBrokerClient;
use crate::operator::{ParkingOperator, StartResponse, StopResponse};
use crate::session::Session;
use std::fmt;

/// VSS signal path for the lock state.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// VSS signal path for parking session active state.
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Error from event processing.
#[derive(Debug)]
pub enum ProcessError {
    /// Session is already active (for start requests).
    AlreadyActive(String),
    /// No active session (for stop requests).
    NoActiveSession,
    /// Operator call failed.
    OperatorFailed(String),
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::AlreadyActive(id) => write!(f, "session already active: {id}"),
            ProcessError::NoActiveSession => write!(f, "no active session"),
            ProcessError::OperatorFailed(msg) => write!(f, "operator call failed: {msg}"),
        }
    }
}

impl std::error::Error for ProcessError {}

/// Process a lock/unlock event from DATA_BROKER.
///
/// - Lock (is_locked=true) + no active session → start session
/// - Lock (is_locked=true) + active session → no-op
/// - Unlock (is_locked=false) + active session → stop session
/// - Unlock (is_locked=false) + no active session → no-op
pub async fn process_lock_event<O: ParkingOperator, B: DataBrokerClient>(
    is_locked: bool,
    session: &mut Session,
    operator: &O,
    broker: &B,
    vehicle_id: &str,
    zone_id: &str,
) -> Result<(), ProcessError> {
    if is_locked {
        // Lock event
        if session.is_active() {
            tracing::info!("lock event received but session already active — no-op");
            return Ok(());
        }
        // Start a new session
        let resp = operator
            .start_session(vehicle_id, zone_id)
            .await
            .map_err(|e| ProcessError::OperatorFailed(e.to_string()))?;

        session.start(
            resp.session_id,
            zone_id.to_string(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            crate::session::Rate {
                rate_type: resp.rate.rate_type,
                amount: resp.rate.amount,
                currency: resp.rate.currency,
            },
        );

        // Publish SessionActive=true, log error on failure
        if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, true).await {
            tracing::error!("failed to publish SessionActive=true: {e}");
        }
    } else {
        // Unlock event
        if !session.is_active() {
            tracing::info!("unlock event received but no active session — no-op");
            return Ok(());
        }
        let session_id = session.status().unwrap().session_id.clone();
        operator
            .stop_session(&session_id)
            .await
            .map_err(|e| ProcessError::OperatorFailed(e.to_string()))?;

        session.stop();

        // Publish SessionActive=false, log error on failure
        if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, false).await {
            tracing::error!("failed to publish SessionActive=false: {e}");
        }
    }
    Ok(())
}

/// Process a manual StartSession request.
///
/// - If session active → return AlreadyActive error
/// - Otherwise → start session with operator, update state, publish signal
pub async fn process_manual_start<O: ParkingOperator, B: DataBrokerClient>(
    zone_id: &str,
    session: &mut Session,
    operator: &O,
    broker: &B,
    vehicle_id: &str,
) -> Result<StartResponse, ProcessError> {
    if session.is_active() {
        let id = session.status().unwrap().session_id.clone();
        return Err(ProcessError::AlreadyActive(id));
    }

    let resp = operator
        .start_session(vehicle_id, zone_id)
        .await
        .map_err(|e| ProcessError::OperatorFailed(e.to_string()))?;

    session.start(
        resp.session_id.clone(),
        zone_id.to_string(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        crate::session::Rate {
            rate_type: resp.rate.rate_type.clone(),
            amount: resp.rate.amount,
            currency: resp.rate.currency.clone(),
        },
    );

    // Publish SessionActive=true, log error on failure
    if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, true).await {
        tracing::error!("failed to publish SessionActive=true: {e}");
    }

    Ok(resp)
}

/// Process a manual StopSession request.
///
/// - If no active session → return NoActiveSession error
/// - Otherwise → stop session with operator, clear state, publish signal
pub async fn process_manual_stop<O: ParkingOperator, B: DataBrokerClient>(
    session: &mut Session,
    operator: &O,
    broker: &B,
) -> Result<StopResponse, ProcessError> {
    if !session.is_active() {
        return Err(ProcessError::NoActiveSession);
    }

    let session_id = session.status().unwrap().session_id.clone();
    let resp = operator
        .stop_session(&session_id)
        .await
        .map_err(|e| ProcessError::OperatorFailed(e.to_string()))?;

    session.stop();

    // Publish SessionActive=false, log error on failure
    if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, false).await {
        tracing::error!("failed to publish SessionActive=false: {e}");
    }

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Rate;
    use crate::testing::{
        make_start_response, make_stop_response, MockBrokerClient, MockOperatorClient,
    };

    fn sample_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        }
    }

    // TS-08-11: Lock Event Starts Session
    #[tokio::test]
    async fn test_lock_event_starts_session() {
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();

        process_lock_event(true, &mut session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1")
            .await
            .expect("lock event should succeed");

        assert_eq!(operator.start_call_count(), 1);
        assert!(session.is_active());
        let calls = broker.set_bool_calls();
        assert!(
            calls.contains(&(SIGNAL_SESSION_ACTIVE.to_string(), true)),
            "should publish SessionActive=true"
        );
    }

    // TS-08-12: Unlock Event Stops Session
    #[tokio::test]
    async fn test_unlock_event_stops_session() {
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();

        // First start a session
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        // Now unlock
        process_lock_event(false, &mut session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1")
            .await
            .expect("unlock event should succeed");

        assert_eq!(operator.stop_call_count(), 1);
        assert!(!session.is_active());
        let calls = broker.set_bool_calls();
        assert!(
            calls.contains(&(SIGNAL_SESSION_ACTIVE.to_string(), false)),
            "should publish SessionActive=false"
        );
    }

    // TS-08-13: SessionActive Set True on Start
    #[tokio::test]
    async fn test_session_active_set_true() {
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();

        process_lock_event(true, &mut session, &operator, &broker, "VIN", "zone")
            .await
            .unwrap();

        let calls = broker.set_bool_calls();
        assert!(calls.contains(&(SIGNAL_SESSION_ACTIVE.to_string(), true)));
    }

    // TS-08-14: SessionActive Set False on Stop
    #[tokio::test]
    async fn test_session_active_set_false() {
        let operator = MockOperatorClient::new();
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        process_lock_event(false, &mut session, &operator, &broker, "VIN", "zone")
            .await
            .unwrap();

        let calls = broker.set_bool_calls();
        assert!(calls.contains(&(SIGNAL_SESSION_ACTIVE.to_string(), false)));
    }

    // TS-08-16: Manual StartSession Override
    #[tokio::test]
    async fn test_manual_start_override() {
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();

        let resp =
            process_manual_start("zone-manual", &mut session, &operator, &broker, "DEMO-VIN-001")
                .await
                .expect("manual start should succeed");

        assert!(!resp.session_id.is_empty());
        assert!(session.is_active());
    }

    // TS-08-17: Manual StopSession Override
    #[tokio::test]
    async fn test_manual_stop_override() {
        let operator = MockOperatorClient::new();
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        let resp = process_manual_stop(&mut session, &operator, &broker)
            .await
            .expect("manual stop should succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert!(!session.is_active());
    }

    // TS-08-2: StartSession RPC Returns Session Info (tested via process_manual_start)
    #[tokio::test]
    async fn test_start_session_rpc_returns_info() {
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();

        let resp =
            process_manual_start("zone-a", &mut session, &operator, &broker, "DEMO-VIN-001")
                .await
                .expect("start should succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.5).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");
        assert!(session.is_active());
    }

    // TS-08-3: StopSession RPC Returns Stop Info (tested via process_manual_stop)
    #[tokio::test]
    async fn test_stop_session_rpc_returns_info() {
        let operator = MockOperatorClient::new();
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        let resp = process_manual_stop(&mut session, &operator, &broker)
            .await
            .expect("stop should succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert!((resp.total_amount - 2.50).abs() < f64::EPSILON);
        assert!(!session.is_active());
    }

    // TS-08-E1: StartSession When Already Active
    #[tokio::test]
    async fn test_start_session_already_active() {
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        let result =
            process_manual_start("zone-b", &mut session, &operator, &broker, "DEMO-VIN-001").await;
        assert!(result.is_err(), "should return error when session already active");

        match result.unwrap_err() {
            ProcessError::AlreadyActive(id) => assert_eq!(id, "sess-1"),
            other => panic!("expected AlreadyActive, got: {other:?}"),
        }
    }

    // TS-08-E2: StopSession When No Session Active
    #[tokio::test]
    async fn test_stop_session_no_active() {
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();
        let mut session = Session::new();

        let result = process_manual_stop(&mut session, &operator, &broker).await;
        assert!(result.is_err(), "should return error when no active session");

        match result.unwrap_err() {
            ProcessError::NoActiveSession => {}
            other => panic!("expected NoActiveSession, got: {other:?}"),
        }
    }

    // TS-08-E6: Lock Event While Session Active (No-op)
    #[tokio::test]
    async fn test_lock_event_noop_when_active() {
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        process_lock_event(true, &mut session, &operator, &broker, "VIN", "zone")
            .await
            .expect("lock event noop should succeed");

        assert_eq!(operator.start_call_count(), 0, "operator should NOT be called");
        assert!(session.is_active());
        assert_eq!(session.status().unwrap().session_id, "sess-1");
    }

    // TS-08-E7: Unlock Event While No Session (No-op)
    #[tokio::test]
    async fn test_unlock_event_noop_when_inactive() {
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();
        let mut session = Session::new();

        process_lock_event(false, &mut session, &operator, &broker, "VIN", "zone")
            .await
            .expect("unlock event noop should succeed");

        assert_eq!(operator.stop_call_count(), 0, "operator should NOT be called");
        assert!(!session.is_active());
    }

    // TS-08-E9: SessionActive Publish Failure
    #[tokio::test]
    async fn test_session_active_publish_failure() {
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();
        broker.fail_set_bool();
        let mut session = Session::new();

        // Session start should still succeed even if broker publish fails
        process_lock_event(true, &mut session, &operator, &broker, "VIN", "zone")
            .await
            .expect("should succeed despite broker publish failure");

        // Memory state should still be correct
        assert!(session.is_active(), "memory state should be active");
    }

    // TS-08-E11: Override Resumes Autonomous on Next Cycle
    #[tokio::test]
    async fn test_override_resumes_autonomous() {
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();
        let mut session = Session::new();

        // Manual start
        process_manual_start("zone-a", &mut session, &operator, &broker, "VIN")
            .await
            .expect("manual start should succeed");
        assert!(session.is_active());

        // Manual stop
        process_manual_stop(&mut session, &operator, &broker)
            .await
            .expect("manual stop should succeed");
        assert!(!session.is_active());

        // Now configure a fresh start response for the autonomous cycle
        operator.on_start_return(make_start_response("sess-2"));

        // Autonomous lock should start a new session
        process_lock_event(true, &mut session, &operator, &broker, "VIN", "zone")
            .await
            .expect("autonomous start should succeed after override");
        assert!(session.is_active());
        assert!(operator.start_call_count() >= 2, "operator start should be called at least twice");
    }
}
