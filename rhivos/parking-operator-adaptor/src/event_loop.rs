use crate::operator::{OperatorError, StartResponse, StopResponse};
use crate::session::Session;

/// Signal path for the SessionActive flag in DATA_BROKER.
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Trait abstracting the PARKING_OPERATOR REST client.
///
/// Allows event processing logic to be tested with a mock operator.
#[allow(async_fn_in_trait)]
pub trait OperatorOps {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError>;

    async fn stop_session(
        &self,
        session_id: &str,
    ) -> Result<StopResponse, OperatorError>;
}

/// Trait abstracting the DATA_BROKER signal publication.
///
/// Allows event processing logic to be tested with a mock broker.
#[allow(async_fn_in_trait)]
pub trait BrokerOps {
    async fn set_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> Result<(), String>;
}

/// Process a lock state change event.
///
/// When `is_locked` is `true` and no session is active, starts a new
/// parking session autonomously. When `is_locked` is `false` and a
/// session is active, stops the session. Duplicate events are no-ops.
///
/// On successful start/stop, publishes `Vehicle.Parking.SessionActive`
/// to DATA_BROKER. Publish failures are logged but do not affect
/// session state.
pub async fn process_lock_event<O: OperatorOps, B: BrokerOps>(
    _session: &mut Session,
    _operator: &O,
    _broker: &B,
    _vehicle_id: &str,
    _zone_id: &str,
    _is_locked: bool,
) {
    todo!("process_lock_event not yet implemented")
}

/// Process a manual StartSession request.
///
/// Returns `Err` with an appropriate gRPC status description if a
/// session is already active (`ALREADY_EXISTS`). Otherwise starts
/// a session and publishes `SessionActive=true`.
pub async fn process_manual_start<O: OperatorOps, B: BrokerOps>(
    _session: &mut Session,
    _operator: &O,
    _broker: &B,
    _vehicle_id: &str,
    _zone_id: &str,
) -> Result<StartResponse, ManualError> {
    todo!("process_manual_start not yet implemented")
}

/// Process a manual StopSession request.
///
/// Returns `Err` with `FAILED_PRECONDITION` if no session is active.
/// Otherwise stops the session and publishes `SessionActive=false`.
pub async fn process_manual_stop<O: OperatorOps, B: BrokerOps>(
    _session: &mut Session,
    _operator: &O,
    _broker: &B,
) -> Result<StopResponse, ManualError> {
    todo!("process_manual_stop not yet implemented")
}

/// Error type for manual gRPC session commands.
#[derive(Debug)]
pub enum ManualError {
    /// StartSession called when a session is already active.
    AlreadyExists(String),
    /// StopSession called when no session is active.
    FailedPrecondition,
    /// Operator REST call failed after retries.
    OperatorUnavailable(String),
}

// Suppress unused import warning until implementation.
const _: () = {
    fn _use_session(_s: &Session) {}
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Rate, Session};
    use crate::testing::{MockBrokerClient, MockOperatorClient};

    fn sample_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    // TS-08-11: Lock Event Starts Session
    // Verify a lock event (IsLocked=true) triggers session start.
    #[tokio::test]
    async fn test_lock_event_starts_session() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");

        process_lock_event(
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
            true,
        )
        .await;

        assert!(session.is_active(), "session should be active after lock event");
        let calls = operator.start_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "DEMO-VIN-001");
        assert_eq!(calls[0].1, "zone-demo-1");
        assert_eq!(
            broker.last_set_bool(),
            Some((SIGNAL_SESSION_ACTIVE.to_string(), true))
        );
    }

    // TS-08-12: Unlock Event Stops Session
    // Verify an unlock event (IsLocked=false) triggers session stop.
    #[tokio::test]
    async fn test_unlock_event_stops_session() {
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        operator.on_stop_success("sess-1", 3600, 2.50, "EUR");

        process_lock_event(&mut session, &operator, &broker, "VIN", "zone", false).await;

        assert!(!session.is_active(), "session should be inactive after unlock event");
        let calls = operator.stop_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "sess-1");
        assert_eq!(
            broker.last_set_bool(),
            Some((SIGNAL_SESSION_ACTIVE.to_string(), false))
        );
    }

    // TS-08-13: SessionActive Set True on Start
    // Verify Vehicle.Parking.SessionActive is set to true when session starts.
    #[tokio::test]
    async fn test_session_active_set_true() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");

        process_lock_event(&mut session, &operator, &broker, "VIN", "zone", true).await;

        let calls = broker.set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(s, v)| s == SIGNAL_SESSION_ACTIVE && *v),
            "should have published SessionActive=true"
        );
    }

    // TS-08-14: SessionActive Set False on Stop
    // Verify Vehicle.Parking.SessionActive is set to false when session stops.
    #[tokio::test]
    async fn test_session_active_set_false() {
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();
        operator.on_stop_success("sess-1", 3600, 2.50, "EUR");

        process_lock_event(&mut session, &operator, &broker, "VIN", "zone", false).await;

        let calls = broker.set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(s, v)| s == SIGNAL_SESSION_ACTIVE && !(*v)),
            "should have published SessionActive=false"
        );
    }

    // TS-08-2: StartSession RPC Returns Session Info
    // Verify manual StartSession calls the operator and returns session info.
    #[tokio::test]
    async fn test_manual_start_session() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");

        let resp = process_manual_start(&mut session, &operator, &broker, "VIN", "zone-a")
            .await
            .expect("manual start should succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");
        assert!(session.is_active());
    }

    // TS-08-3: StopSession RPC Returns Stop Info
    // Verify manual StopSession calls the operator and returns stop info.
    #[tokio::test]
    async fn test_manual_stop_session() {
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();
        operator.on_stop_success("sess-1", 3600, 2.50, "EUR");

        let resp = process_manual_stop(&mut session, &operator, &broker)
            .await
            .expect("manual stop should succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert!((resp.total_amount - 2.50).abs() < f64::EPSILON);
        assert!(!session.is_active());
    }

    // TS-08-16: Manual StartSession Override
    // Verify manual StartSession works regardless of lock state.
    #[tokio::test]
    async fn test_manual_start_override() {
        let mut session = Session::new();
        // Lock state doesn't matter — manual start works regardless.
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");

        let resp = process_manual_start(&mut session, &operator, &broker, "VIN", "zone-manual")
            .await
            .expect("manual start should work regardless of lock state");

        assert!(!resp.session_id.is_empty());
        assert!(session.is_active());
    }

    // TS-08-17: Manual StopSession Override
    // Verify manual StopSession works regardless of lock state.
    #[tokio::test]
    async fn test_manual_stop_override() {
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();
        operator.on_stop_success("sess-1", 3600, 2.50, "EUR");

        let resp = process_manual_stop(&mut session, &operator, &broker)
            .await
            .expect("manual stop should work regardless of lock state");

        assert_eq!(resp.session_id, "sess-1");
        assert!(!session.is_active());
    }

    // TS-08-E1: StartSession When Already Active
    // Verify StartSession returns AlreadyExists when a session is active.
    #[tokio::test]
    async fn test_start_session_already_active() {
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        let result =
            process_manual_start(&mut session, &operator, &broker, "VIN", "zone-b").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ManualError::AlreadyExists(id) => assert_eq!(id, "sess-1"),
            other => panic!("expected AlreadyExists, got {other:?}"),
        }
    }

    // TS-08-E2: StopSession When No Session Active
    // Verify StopSession returns FailedPrecondition when no session.
    #[tokio::test]
    async fn test_stop_session_no_active() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        let result = process_manual_stop(&mut session, &operator, &broker).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ManualError::FailedPrecondition => {} // expected
            other => panic!("expected FailedPrecondition, got {other:?}"),
        }
    }

    // TS-08-E6: Lock Event While Session Active (No-op)
    // Verify lock event during active session is a no-op.
    #[tokio::test]
    async fn test_lock_event_noop_when_active() {
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        process_lock_event(&mut session, &operator, &broker, "VIN", "zone", true).await;

        assert_eq!(operator.start_call_count(), 0, "should not call start");
        assert!(session.is_active());
        assert_eq!(
            session.status().unwrap().session_id,
            "sess-1",
            "session_id should be unchanged"
        );
    }

    // TS-08-E7: Unlock Event While No Session (No-op)
    // Verify unlock event without active session is a no-op.
    #[tokio::test]
    async fn test_unlock_event_noop_when_inactive() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        process_lock_event(&mut session, &operator, &broker, "VIN", "zone", false).await;

        assert_eq!(operator.stop_call_count(), 0, "should not call stop");
        assert!(!session.is_active());
    }

    // TS-08-E9: SessionActive Publish Failure
    // Verify the service continues after failing to publish SessionActive.
    #[tokio::test]
    async fn test_session_active_publish_failure() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();
        broker.fail_set_bool();

        operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");

        process_lock_event(&mut session, &operator, &broker, "VIN", "zone", true).await;

        // Session state should still be updated in memory despite publish failure.
        assert!(session.is_active(), "memory state should be active");

        // Service should still process subsequent events.
        let operator2 = MockOperatorClient::new();
        let broker2 = MockBrokerClient::new();
        operator2.on_stop_success("sess-1", 3600, 2.50, "EUR");

        process_lock_event(&mut session, &operator2, &broker2, "VIN", "zone", false).await;
        assert!(!session.is_active(), "should process subsequent events");
    }

    // TS-08-E11: Override Resumes Autonomous on Next Cycle
    // Verify autonomous behavior resumes after manual override.
    #[tokio::test]
    async fn test_override_resumes_autonomous() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        // Step 1: Manual start.
        operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");
        let _ = process_manual_start(&mut session, &operator, &broker, "VIN", "zone-a").await;
        assert!(session.is_active());

        // Step 2: Manual stop.
        let operator2 = MockOperatorClient::new();
        let broker2 = MockBrokerClient::new();
        operator2.on_stop_success("sess-1", 3600, 2.50, "EUR");
        let _ = process_manual_stop(&mut session, &operator2, &broker2).await;
        assert!(!session.is_active());

        // Step 3: Autonomous lock event should start new session.
        let operator3 = MockOperatorClient::new();
        let broker3 = MockBrokerClient::new();
        operator3.on_start_success("sess-2", "per_hour", 2.50, "EUR");
        process_lock_event(&mut session, &operator3, &broker3, "VIN", "zone", true).await;
        assert!(session.is_active(), "autonomous should resume after override");
        assert_eq!(operator3.start_call_count(), 1);
    }

    // TS-08-E14: Concurrent Lock Event and Manual StopSession
    // Verify both operations are processed without state corruption.
    // In unit tests we serialize manually — the sequential processing
    // property is tested via proptest (TS-08-P6).
    #[tokio::test]
    async fn test_concurrent_lock_and_stop_serialized() {
        let mut session = Session::new();
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, sample_rate());

        // Process lock event first (no-op since already active).
        let op1 = MockOperatorClient::new();
        let br1 = MockBrokerClient::new();
        process_lock_event(&mut session, &op1, &br1, "VIN", "zone", true).await;
        assert!(session.is_active(), "lock event is no-op when active");

        // Then process manual stop.
        let op2 = MockOperatorClient::new();
        let br2 = MockBrokerClient::new();
        op2.on_stop_success("sess-1", 3600, 2.50, "EUR");
        let result = process_manual_stop(&mut session, &op2, &br2).await;
        assert!(result.is_ok());
        assert!(!session.is_active(), "manual stop should deactivate session");
    }
}
