//! Event processing loop.
//!
//! Serialises lock/unlock events from DATA_BROKER and manual gRPC commands
//! into a single processing stream, preventing race conditions on session state
//! (08-REQ-9.1, 08-REQ-9.2).

#![allow(dead_code)]
#![allow(async_fn_in_trait)]

use crate::operator::{OperatorError, StartResponse, StopResponse};
use crate::session::Session;

// ── Traits ────────────────────────────────────────────────────────────────────

/// Abstraction over the PARKING_OPERATOR REST client for testability.
pub trait OperatorTrait: Send + Sync {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError>;
    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError>;
}

/// Abstraction over DATA_BROKER for publishing `Vehicle.Parking.SessionActive`.
pub trait BrokerTrait: Send + Sync {
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
}

/// Error returned by the broker abstraction.
#[derive(Debug)]
pub struct BrokerError(pub String);

/// Signal path for the session-active VSS signal.
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";
/// Signal path for the door lock VSS signal.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

// ── Processing functions ──────────────────────────────────────────────────────

/// Process a lock/unlock event from DATA_BROKER.
///
/// Lock event (`is_locked = true`):
/// - If session active → log info, no-op (08-REQ-3.E1).
/// - Otherwise → start session with operator, update state, publish signal (08-REQ-3.3, 08-REQ-4.1).
///
/// Unlock event (`is_locked = false`):
/// - If no session active → log info, no-op (08-REQ-3.E2).
/// - Otherwise → stop session with operator, clear state, publish signal (08-REQ-3.4, 08-REQ-4.2).
pub async fn process_lock_event<O, B>(
    _is_locked: bool,
    _session: &mut Session,
    _operator: &O,
    _broker: &B,
    _vehicle_id: &str,
    _zone_id: &str,
) -> Result<(), ()>
where
    O: OperatorTrait,
    B: BrokerTrait,
{
    todo!("implement process_lock_event — start on lock, stop on unlock, with idempotency")
}

/// Process a manual StartSession gRPC command (08-REQ-5.1).
///
/// Returns `ALREADY_EXISTS` if a session is already active (08-REQ-1.E1).
pub async fn process_manual_start<O, B>(
    _zone_id: &str,
    _session: &mut Session,
    _operator: &O,
    _broker: &B,
    _vehicle_id: &str,
) -> Result<StartResponse, tonic::Status>
where
    O: OperatorTrait,
    B: BrokerTrait,
{
    todo!("implement process_manual_start — start session, return ALREADY_EXISTS if active")
}

/// Process a manual StopSession gRPC command (08-REQ-5.2).
///
/// Returns `FAILED_PRECONDITION` if no session is active (08-REQ-1.E2).
pub async fn process_manual_stop<O, B>(
    _session: &mut Session,
    _operator: &O,
    _broker: &B,
) -> Result<StopResponse, tonic::Status>
where
    O: OperatorTrait,
    B: BrokerTrait,
{
    todo!("implement process_manual_stop — stop session, return FAILED_PRECONDITION if none active")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::{OperatorError, StartResponse, StopResponse};
    use crate::session::{Rate, Session};
    use std::sync::Mutex;

    // ── Mock types ────────────────────────────────────────────────────────────

    /// Mock PARKING_OPERATOR client for unit testing.
    struct MockOperatorClient {
        start_response: Option<StartResponse>,
        stop_response: Option<StopResponse>,
        start_call_count: Mutex<usize>,
        stop_call_count: Mutex<usize>,
        always_fail: bool,
    }

    impl MockOperatorClient {
        fn new() -> Self {
            MockOperatorClient {
                start_response: None,
                stop_response: None,
                start_call_count: Mutex::new(0),
                stop_call_count: Mutex::new(0),
                always_fail: false,
            }
        }

        fn with_start_response(mut self, r: StartResponse) -> Self {
            self.start_response = Some(r);
            self
        }

        fn with_stop_response(mut self, r: StopResponse) -> Self {
            self.stop_response = Some(r);
            self
        }

        fn with_always_fail(mut self) -> Self {
            self.always_fail = true;
            self
        }

        fn start_call_count(&self) -> usize {
            *self.start_call_count.lock().unwrap()
        }

        fn stop_call_count(&self) -> usize {
            *self.stop_call_count.lock().unwrap()
        }
    }

    impl OperatorTrait for MockOperatorClient {
        async fn start_session(
            &self,
            _vehicle_id: &str,
            _zone_id: &str,
        ) -> Result<StartResponse, OperatorError> {
            *self.start_call_count.lock().unwrap() += 1;
            if self.always_fail {
                return Err(OperatorError::RetriesExhausted("mock failure".to_string()));
            }
            Ok(self
                .start_response
                .clone()
                .expect("MockOperatorClient: start_response not configured"))
        }

        async fn stop_session(&self, _session_id: &str) -> Result<StopResponse, OperatorError> {
            *self.stop_call_count.lock().unwrap() += 1;
            if self.always_fail {
                return Err(OperatorError::RetriesExhausted("mock failure".to_string()));
            }
            Ok(self
                .stop_response
                .clone()
                .expect("MockOperatorClient: stop_response not configured"))
        }
    }

    /// Mock DATA_BROKER client for unit testing.
    struct MockBrokerClient {
        set_bool_calls: Mutex<Vec<(String, bool)>>,
        fail_set_bool: bool,
    }

    impl MockBrokerClient {
        fn new() -> Self {
            MockBrokerClient {
                set_bool_calls: Mutex::new(Vec::new()),
                fail_set_bool: false,
            }
        }

        fn with_fail_set_bool(mut self) -> Self {
            self.fail_set_bool = true;
            self
        }

        fn last_set_bool(&self) -> Option<(String, bool)> {
            self.set_bool_calls.lock().unwrap().last().cloned()
        }

        fn all_set_bool_calls(&self) -> Vec<(String, bool)> {
            self.set_bool_calls.lock().unwrap().clone()
        }
    }

    impl BrokerTrait for MockBrokerClient {
        async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
            self.set_bool_calls
                .lock()
                .unwrap()
                .push((signal.to_string(), value));
            if self.fail_set_bool {
                return Err(BrokerError("mock broker failure".to_string()));
            }
            Ok(())
        }
    }

    // ── Test helpers ──────────────────────────────────────────────────────────

    fn make_rate(rate_type: &str, amount: f64, currency: &str) -> Rate {
        Rate {
            rate_type: rate_type.to_string(),
            amount,
            currency: currency.to_string(),
        }
    }

    fn make_start_response(session_id: &str) -> StartResponse {
        StartResponse {
            session_id: session_id.to_string(),
            status: "active".to_string(),
            rate: make_rate("per_hour", 2.5, "EUR"),
        }
    }

    fn make_stop_response(session_id: &str) -> StopResponse {
        StopResponse {
            session_id: session_id.to_string(),
            status: "completed".to_string(),
            duration_seconds: 3600,
            total_amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    // ── Task 1.4 tests ────────────────────────────────────────────────────────

    /// TS-08-11: Lock event triggers session start.
    ///
    /// Requires: 08-REQ-3.3, 08-REQ-4.1
    #[tokio::test]
    async fn test_lock_event_starts_session() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new().with_start_response(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();

        process_lock_event(true, &mut session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1")
            .await
            .expect("lock event must succeed");

        assert!(session.is_active(), "session must be active after lock");
        assert_eq!(
            session.status().unwrap().session_id,
            "sess-1",
            "session_id must match operator response"
        );
        assert_eq!(operator.start_call_count(), 1, "start must be called once");
        let last = broker.last_set_bool().expect("broker must receive set_bool call");
        assert_eq!(last.0, SIGNAL_SESSION_ACTIVE);
        assert!(last.1, "SessionActive must be set to true");
    }

    /// TS-08-12: Unlock event triggers session stop.
    ///
    /// Requires: 08-REQ-3.4, 08-REQ-4.2
    #[tokio::test]
    async fn test_unlock_event_stops_session() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            make_rate("per_hour", 2.5, "EUR"),
        );
        let operator = MockOperatorClient::new().with_stop_response(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        process_lock_event(false, &mut session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1")
            .await
            .expect("unlock event must succeed");

        assert!(!session.is_active(), "session must be inactive after unlock");
        assert_eq!(operator.stop_call_count(), 1, "stop must be called once");
        let last = broker.last_set_bool().expect("broker must receive set_bool call");
        assert_eq!(last.0, SIGNAL_SESSION_ACTIVE);
        assert!(!last.1, "SessionActive must be set to false");
    }

    /// TS-08-13: Vehicle.Parking.SessionActive set to true when session starts.
    ///
    /// Requires: 08-REQ-4.1
    #[tokio::test]
    async fn test_session_active_set_true() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new().with_start_response(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();

        process_lock_event(true, &mut session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        let calls = broker.all_set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(sig, val)| sig == SIGNAL_SESSION_ACTIVE && *val),
            "set_bool(SessionActive, true) must be called"
        );
    }

    /// TS-08-14: Vehicle.Parking.SessionActive set to false when session stops.
    ///
    /// Requires: 08-REQ-4.2
    #[tokio::test]
    async fn test_session_active_set_false() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            make_rate("per_hour", 2.5, "EUR"),
        );
        let operator = MockOperatorClient::new().with_stop_response(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        process_lock_event(false, &mut session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        let calls = broker.all_set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(sig, val)| sig == SIGNAL_SESSION_ACTIVE && !*val),
            "set_bool(SessionActive, false) must be called"
        );
    }

    /// TS-08-2 / TS-08-16: Manual StartSession works regardless of lock state.
    ///
    /// Requires: 08-REQ-5.1, 08-REQ-1.2
    #[tokio::test]
    async fn test_manual_start_override() {
        let mut session = Session::new();
        let operator =
            MockOperatorClient::new().with_start_response(make_start_response("sess-manual"));
        let broker = MockBrokerClient::new();

        let resp = process_manual_start("zone-manual", &mut session, &operator, &broker, "VIN")
            .await
            .expect("manual start must succeed");

        assert!(!resp.session_id.is_empty(), "session_id must be non-empty");
        assert!(session.is_active(), "session must be active after manual start");
        assert_eq!(resp.session_id, "sess-manual");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert_eq!(resp.rate.amount, 2.5);
        assert_eq!(resp.rate.currency, "EUR");
    }

    /// TS-08-3 / TS-08-17: Manual StopSession works regardless of lock state.
    ///
    /// Requires: 08-REQ-5.2, 08-REQ-1.3
    #[tokio::test]
    async fn test_manual_stop_override() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            make_rate("per_hour", 2.5, "EUR"),
        );
        let operator = MockOperatorClient::new().with_stop_response(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        let resp = process_manual_stop(&mut session, &operator, &broker)
            .await
            .expect("manual stop must succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert_eq!(resp.total_amount, 2.50);
        assert!(!session.is_active(), "session must be inactive after manual stop");
    }

    // ── Task 1.5 edge case tests ──────────────────────────────────────────────

    /// TS-08-E1: StartSession when already active returns ALREADY_EXISTS.
    ///
    /// Requires: 08-REQ-1.E1
    #[tokio::test]
    async fn test_start_session_already_active() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            make_rate("per_hour", 2.5, "EUR"),
        );
        let operator =
            MockOperatorClient::new().with_start_response(make_start_response("sess-new"));
        let broker = MockBrokerClient::new();

        let result = process_manual_start("zone-b", &mut session, &operator, &broker, "VIN").await;

        assert!(result.is_err(), "must return error when session already active");
        assert_eq!(
            result.unwrap_err().code(),
            tonic::Code::AlreadyExists,
            "error code must be ALREADY_EXISTS"
        );
        // Operator must NOT be called.
        assert_eq!(operator.start_call_count(), 0, "operator must not be called");
    }

    /// TS-08-E2: StopSession when no session active returns FAILED_PRECONDITION.
    ///
    /// Requires: 08-REQ-1.E2
    #[tokio::test]
    async fn test_stop_session_no_active() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new().with_stop_response(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        let result = process_manual_stop(&mut session, &operator, &broker).await;

        assert!(result.is_err(), "must return error when no session active");
        assert_eq!(
            result.unwrap_err().code(),
            tonic::Code::FailedPrecondition,
            "error code must be FAILED_PRECONDITION"
        );
        assert_eq!(operator.stop_call_count(), 0, "operator must not be called");
    }

    /// TS-08-E6: Lock event is a no-op when session already active.
    ///
    /// Requires: 08-REQ-3.E1
    #[tokio::test]
    async fn test_lock_event_noop_when_active() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            make_rate("per_hour", 2.5, "EUR"),
        );
        let operator =
            MockOperatorClient::new().with_start_response(make_start_response("sess-new"));
        let broker = MockBrokerClient::new();

        process_lock_event(true, &mut session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        // Session unchanged, operator not called.
        assert_eq!(operator.start_call_count(), 0, "operator must NOT be called");
        assert!(session.is_active(), "session must remain active");
        assert_eq!(
            session.status().unwrap().session_id,
            "sess-1",
            "session_id must be unchanged"
        );
    }

    /// TS-08-E7: Unlock event is a no-op when no session is active.
    ///
    /// Requires: 08-REQ-3.E2
    #[tokio::test]
    async fn test_unlock_event_noop_when_inactive() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new().with_stop_response(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        process_lock_event(false, &mut session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        assert_eq!(operator.stop_call_count(), 0, "operator must NOT be called");
        assert!(!session.is_active(), "session must remain inactive");
    }

    /// TS-08-E9: Broker publish failure does not corrupt session state.
    ///
    /// Requires: 08-REQ-4.E1
    #[tokio::test]
    async fn test_session_active_publish_failure() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new().with_start_response(make_start_response("sess-1"));
        // Broker configured to fail set_bool calls.
        let broker = MockBrokerClient::new().with_fail_set_bool();

        // Should succeed despite broker failure (08-REQ-4.E1: log and continue).
        let result =
            process_lock_event(true, &mut session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1")
                .await;

        assert!(result.is_ok(), "must continue despite broker failure");
        // Session state must still be updated in memory.
        assert!(session.is_active(), "session must be active in memory");
    }

    /// TS-08-E11: Override does not persist — autonomous behavior resumes on next cycle.
    ///
    /// Requires: 08-REQ-5.3, 08-REQ-5.E1
    #[tokio::test]
    async fn test_override_resumes_autonomous() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new()
            .with_start_response(make_start_response("sess-1"))
            .with_stop_response(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        // Manual start.
        process_manual_start("zone-a", &mut session, &operator, &broker, "VIN")
            .await
            .expect("manual start must succeed");
        assert!(session.is_active());

        // Manual stop.
        process_manual_stop(&mut session, &operator, &broker)
            .await
            .expect("manual stop must succeed");
        assert!(!session.is_active());

        // Autonomous lock event must start new session.
        let operator2 =
            MockOperatorClient::new().with_start_response(make_start_response("sess-2"));
        let broker2 = MockBrokerClient::new();

        process_lock_event(true, &mut session, &operator2, &broker2, "DEMO-VIN-001", "zone-demo-1")
            .await
            .expect("autonomous start after override must succeed");

        assert!(session.is_active(), "autonomous start must succeed after override");
        assert_eq!(operator2.start_call_count(), 1, "operator must be called for autonomous start");
    }
}
