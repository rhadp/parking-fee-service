//! Sequential event processing for lock/unlock events and gRPC commands.
//!
//! All events are serialized through `process_lock_event`, `manual_start`,
//! and `manual_stop` — ensuring no concurrent mutations of session state.

use crate::broker::SessionPublisher;
use crate::operator::{OperatorApi, StartResponse, StopResponse};
use crate::session::Session;

/// Errors from event processing.
#[derive(Debug, PartialEq)]
pub enum EventError {
    /// `StartSession` called while a session is already active (08-REQ-1.E1).
    AlreadyActive { existing_session_id: String },
    /// `StopSession` called when no session is active (08-REQ-1.E2).
    NotActive,
    /// Operator REST call failed after all retries.
    OperatorFailed(String),
}

/// Process a lock/unlock event from DATA_BROKER.
///
/// - `IsLocked = true`:  start session (no-op if already active, 08-REQ-3.E1).
/// - `IsLocked = false`: stop session  (no-op if not active,  08-REQ-3.E2).
///
/// On success the DATA_BROKER signal `Vehicle.Parking.SessionActive` is updated.
/// If that publish fails, the error is logged and operation continues (08-REQ-4.E1).
pub async fn process_lock_event(
    _is_locked: bool,
    _session: &mut Session,
    _operator: &impl OperatorApi,
    _publisher: &impl SessionPublisher,
    _vehicle_id: &str,
    _zone_id: &str,
) -> Result<(), EventError> {
    todo!("process_lock_event not yet implemented")
}

/// Handle a manual `StartSession` gRPC call (08-REQ-5.1).
///
/// Returns `EventError::AlreadyActive` when a session is already active.
pub async fn manual_start(
    _zone_id: &str,
    _session: &mut Session,
    _operator: &impl OperatorApi,
    _publisher: &impl SessionPublisher,
    _vehicle_id: &str,
) -> Result<StartResponse, EventError> {
    todo!("manual_start not yet implemented")
}

/// Handle a manual `StopSession` gRPC call (08-REQ-5.2).
///
/// Returns `EventError::NotActive` when no session is active.
pub async fn manual_stop(
    _session: &mut Session,
    _operator: &impl OperatorApi,
    _publisher: &impl SessionPublisher,
) -> Result<StopResponse, EventError> {
    todo!("manual_stop not yet implemented")
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::broker::BrokerError;
    use crate::operator::{OperatorError, RateResponse};
    use crate::session::Rate;

    // ── Mock operator ─────────────────────────────────────────────────────────

    struct MockOperator {
        start_ok: bool,
        stop_ok: bool,
        start_call_count: RefCell<usize>,
        stop_call_count: RefCell<usize>,
        last_start_vehicle_id: RefCell<Option<String>>,
        last_start_zone_id: RefCell<Option<String>>,
        last_stop_session_id: RefCell<Option<String>>,
    }

    impl MockOperator {
        fn success() -> Self {
            Self {
                start_ok: true,
                stop_ok: true,
                start_call_count: RefCell::new(0),
                stop_call_count: RefCell::new(0),
                last_start_vehicle_id: RefCell::new(None),
                last_start_zone_id: RefCell::new(None),
                last_stop_session_id: RefCell::new(None),
            }
        }

        fn fail_all() -> Self {
            Self {
                start_ok: false,
                stop_ok: false,
                ..Self::success()
            }
        }

        fn start_count(&self) -> usize {
            *self.start_call_count.borrow()
        }

        fn stop_count(&self) -> usize {
            *self.stop_call_count.borrow()
        }
    }

    fn make_start_response(session_id: &str) -> StartResponse {
        StartResponse {
            session_id: session_id.to_owned(),
            status: "active".to_owned(),
            rate: RateResponse {
                rate_type: "per_hour".to_owned(),
                amount: 2.5,
                currency: "EUR".to_owned(),
            },
        }
    }

    fn make_stop_response(session_id: &str) -> StopResponse {
        StopResponse {
            session_id: session_id.to_owned(),
            status: "completed".to_owned(),
            duration_seconds: 3600,
            total_amount: 2.5,
            currency: "EUR".to_owned(),
        }
    }

    impl OperatorApi for MockOperator {
        async fn start_session(
            &self,
            vehicle_id: &str,
            zone_id: &str,
        ) -> Result<StartResponse, OperatorError> {
            *self.start_call_count.borrow_mut() += 1;
            *self.last_start_vehicle_id.borrow_mut() = Some(vehicle_id.to_owned());
            *self.last_start_zone_id.borrow_mut() = Some(zone_id.to_owned());
            if self.start_ok {
                Ok(make_start_response("sess-1"))
            } else {
                Err(OperatorError::RetriesExhausted)
            }
        }

        async fn stop_session(
            &self,
            session_id: &str,
        ) -> Result<StopResponse, OperatorError> {
            *self.stop_call_count.borrow_mut() += 1;
            *self.last_stop_session_id.borrow_mut() = Some(session_id.to_owned());
            if self.stop_ok {
                Ok(make_stop_response(session_id))
            } else {
                Err(OperatorError::RetriesExhausted)
            }
        }
    }

    // ── Mock publisher ────────────────────────────────────────────────────────

    struct MockPublisher {
        fail: bool,
        set_calls: RefCell<Vec<bool>>,
    }

    impl MockPublisher {
        fn success() -> Self {
            Self {
                fail: false,
                set_calls: RefCell::new(Vec::new()),
            }
        }

        fn failing() -> Self {
            Self {
                fail: true,
                set_calls: RefCell::new(Vec::new()),
            }
        }

        fn last_value(&self) -> Option<bool> {
            self.set_calls.borrow().last().copied()
        }
    }

    impl SessionPublisher for MockPublisher {
        async fn set_session_active(&self, active: bool) -> Result<(), BrokerError> {
            self.set_calls.borrow_mut().push(active);
            if self.fail {
                Err(BrokerError::PublishFailed("injected failure".to_owned()))
            } else {
                Ok(())
            }
        }
    }

    // ── TS-08-11: Lock Event Starts Session ───────────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_lock_event_starts_session() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        process_lock_event(
            true,
            &mut session,
            &operator,
            &publisher,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await
        .expect("lock event should start session");

        assert!(session.is_active());
        assert_eq!(operator.start_count(), 1);
        assert_eq!(
            operator.last_start_vehicle_id.borrow().as_deref(),
            Some("DEMO-VIN-001")
        );
        assert_eq!(
            operator.last_start_zone_id.borrow().as_deref(),
            Some("zone-demo-1")
        );
        assert_eq!(publisher.last_value(), Some(true));
    }

    // ── TS-08-12: Unlock Event Stops Session ──────────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_unlock_event_stops_session() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        // Pre-populate an active session.
        session.start(
            "sess-1".to_owned(),
            "zone-a".to_owned(),
            1_000,
            Rate { rate_type: "per_hour".to_owned(), amount: 2.5, currency: "EUR".to_owned() },
        );

        process_lock_event(false, &mut session, &operator, &publisher, "VIN", "zone")
            .await
            .expect("unlock event should stop session");

        assert!(!session.is_active());
        assert_eq!(operator.stop_count(), 1);
        assert_eq!(
            operator.last_stop_session_id.borrow().as_deref(),
            Some("sess-1")
        );
        assert_eq!(publisher.last_value(), Some(false));
    }

    // ── TS-08-13: SessionActive Set True on Start ─────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_session_active_set_true() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        process_lock_event(true, &mut session, &operator, &publisher, "VIN", "zone")
            .await
            .expect("should start session");

        assert_eq!(publisher.last_value(), Some(true));
    }

    // ── TS-08-14: SessionActive Set False on Stop ─────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_session_active_set_false() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        session.start(
            "sess-1".to_owned(),
            "zone-a".to_owned(),
            1_000,
            Rate { rate_type: "per_hour".to_owned(), amount: 2.5, currency: "EUR".to_owned() },
        );

        process_lock_event(false, &mut session, &operator, &publisher, "VIN", "zone")
            .await
            .expect("should stop session");

        assert_eq!(publisher.last_value(), Some(false));
    }

    // ── TS-08-2 / TS-08-16: Manual StartSession ───────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_manual_start_override() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        let resp = manual_start("zone-manual", &mut session, &operator, &publisher, "VIN")
            .await
            .expect("manual start should succeed");

        assert!(!resp.session_id.is_empty());
        assert!(session.is_active());
        assert_eq!(operator.start_count(), 1);
    }

    // ── TS-08-3 / TS-08-17: Manual StopSession ───────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_manual_stop_override() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        session.start(
            "sess-1".to_owned(),
            "zone-a".to_owned(),
            1_000,
            Rate { rate_type: "per_hour".to_owned(), amount: 2.5, currency: "EUR".to_owned() },
        );

        let resp = manual_stop(&mut session, &operator, &publisher)
            .await
            .expect("manual stop should succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert!(!session.is_active());
    }

    // ── TS-08-E1: StartSession When Already Active ────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_start_session_already_active() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        session.start(
            "sess-1".to_owned(),
            "zone-a".to_owned(),
            1_000,
            Rate { rate_type: "per_hour".to_owned(), amount: 2.5, currency: "EUR".to_owned() },
        );

        let result = manual_start("zone-b", &mut session, &operator, &publisher, "VIN").await;

        assert!(result.is_err(), "should fail when session already active");
        assert!(
            matches!(result.unwrap_err(), EventError::AlreadyActive { .. }),
            "error should be AlreadyActive"
        );
        // Operator must NOT have been called.
        assert_eq!(operator.start_count(), 0);
    }

    // ── TS-08-E2: StopSession When No Session Active ──────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_stop_session_no_active() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        let result = manual_stop(&mut session, &operator, &publisher).await;

        assert!(result.is_err(), "should fail when no session active");
        assert_eq!(result.unwrap_err(), EventError::NotActive);
        assert_eq!(operator.stop_count(), 0);
    }

    // ── TS-08-E6: Lock Event No-op When Session Active ────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_lock_event_noop_when_active() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        session.start(
            "sess-1".to_owned(),
            "zone-a".to_owned(),
            1_000,
            Rate { rate_type: "per_hour".to_owned(), amount: 2.5, currency: "EUR".to_owned() },
        );

        // Lock event while session already active — should be a no-op.
        process_lock_event(true, &mut session, &operator, &publisher, "VIN", "zone")
            .await
            .expect("lock event should not error");

        // Operator start must NOT have been called.
        assert_eq!(operator.start_count(), 0);
        assert!(session.is_active());
        // SessionActive must NOT have been published again.
        assert!(publisher.set_calls.borrow().is_empty());
    }

    // ── TS-08-E7: Unlock Event No-op When No Session ─────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_unlock_event_noop_when_inactive() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        // No active session — unlock should be a no-op.
        process_lock_event(false, &mut session, &operator, &publisher, "VIN", "zone")
            .await
            .expect("unlock event should not error");

        assert_eq!(operator.stop_count(), 0);
        assert!(!session.is_active());
        assert!(publisher.set_calls.borrow().is_empty());
    }

    // ── TS-08-E9: SessionActive Publish Failure Continues ────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_session_active_publish_failure() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::failing();
        let mut session = Session::new();

        // Even if publish fails, session start should succeed (memory state is authoritative).
        process_lock_event(true, &mut session, &operator, &publisher, "VIN", "zone")
            .await
            .expect("session should start despite broker publish failure");

        assert!(session.is_active(), "in-memory state must still be active");

        // Subsequent events should still be processed (no panic/crash).
        process_lock_event(false, &mut session, &operator, &publisher, "VIN", "zone")
            .await
            .expect("unlock should succeed even after prior publish failure");
    }

    // ── TS-08-E11: Override Resumes Autonomous on Next Cycle ─────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn test_override_resumes_autonomous() {
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        // Manual start.
        manual_start("zone-a", &mut session, &operator, &publisher, "VIN")
            .await
            .expect("manual start should succeed");
        assert!(session.is_active());

        // Manual stop.
        manual_stop(&mut session, &operator, &publisher)
            .await
            .expect("manual stop should succeed");
        assert!(!session.is_active());

        // Autonomous lock event should start a new session.
        process_lock_event(true, &mut session, &operator, &publisher, "VIN", "zone-demo-1")
            .await
            .expect("autonomous start should succeed after manual stop");

        assert!(session.is_active());
        // Two start calls total: one manual, one autonomous.
        assert_eq!(operator.start_count(), 2);
    }
}
