//! Event processing loop for parking session management.
//!
//! Serialises lock/unlock events (from DATA_BROKER) and manual gRPC commands
//! into sequential operations so that session state is never modified
//! concurrently (08-REQ-9.1, 08-REQ-9.2).

use crate::broker::SessionPublisher;
use crate::operator::{OperatorApi, OperatorError, StartResponse, StopResponse};
use crate::session::Session;

/// Error type for [`manual_start`].
#[derive(Debug)]
pub enum StartError {
    /// A session is already active; contains the existing session_id.
    AlreadyActive { session_id: String },
    /// The PARKING_OPERATOR REST call failed (after all retries).
    Operator(OperatorError),
}

/// Error type for [`manual_stop`].
#[derive(Debug)]
pub enum StopError {
    /// No session is currently active.
    NotActive,
    /// The PARKING_OPERATOR REST call failed (after all retries).
    Operator(OperatorError),
}

/// Process a lock/unlock event from DATA_BROKER.
///
/// - `is_locked = true` → start a new session (no-op if already active).
/// - `is_locked = false` → stop the active session (no-op if none active).
///
/// On failure to publish `Vehicle.Parking.SessionActive`, the error is logged
/// and the in-memory session state remains authoritative (08-REQ-4.E1).
pub async fn process_lock_event(
    is_locked: bool,
    session: &mut Session,
    operator: &dyn OperatorApi,
    publisher: &dyn SessionPublisher,
    vehicle_id: &str,
    zone_id: &str,
) -> Result<(), OperatorError> {
    if is_locked {
        // Lock event → start session
        if session.is_active() {
            tracing::info!("Lock event received but session already active (no-op)");
            return Ok(());
        }

        let resp = operator.start_session(vehicle_id, zone_id).await?;
        let start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        session.start(resp.session_id, zone_id.to_string(), start_time, resp.rate);

        if let Err(e) = publisher.set_session_active(true).await {
            tracing::error!("Failed to publish SessionActive=true: {e}");
        }
    } else {
        // Unlock event → stop session
        if !session.is_active() {
            tracing::info!("Unlock event received but no session is active (no-op)");
            return Ok(());
        }

        let session_id = session.status().unwrap().session_id.clone();
        operator.stop_session(&session_id).await?;
        session.stop();

        if let Err(e) = publisher.set_session_active(false).await {
            tracing::error!("Failed to publish SessionActive=false: {e}");
        }
    }

    Ok(())
}

/// Manually start a parking session via gRPC `StartSession` RPC.
///
/// Returns `StartError::AlreadyActive` immediately if a session is active.
/// Otherwise calls the PARKING_OPERATOR and updates session state.
pub async fn manual_start(
    zone_id: &str,
    session: &mut Session,
    operator: &dyn OperatorApi,
    publisher: &dyn SessionPublisher,
    vehicle_id: &str,
) -> Result<StartResponse, StartError> {
    if session.is_active() {
        let existing = session.status().unwrap().session_id.clone();
        return Err(StartError::AlreadyActive { session_id: existing });
    }

    let resp = operator
        .start_session(vehicle_id, zone_id)
        .await
        .map_err(StartError::Operator)?;

    let start_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    session.start(
        resp.session_id.clone(),
        zone_id.to_string(),
        start_time,
        resp.rate.clone(),
    );

    if let Err(e) = publisher.set_session_active(true).await {
        tracing::error!("Failed to publish SessionActive=true: {e}");
    }

    Ok(resp)
}

/// Manually stop the active parking session via gRPC `StopSession` RPC.
///
/// Returns `StopError::NotActive` if no session is active.
/// Otherwise calls the PARKING_OPERATOR and clears session state.
pub async fn manual_stop(
    session: &mut Session,
    operator: &dyn OperatorApi,
    publisher: &dyn SessionPublisher,
) -> Result<StopResponse, StopError> {
    if !session.is_active() {
        return Err(StopError::NotActive);
    }

    let session_id = session.status().unwrap().session_id.clone();
    let resp = operator
        .stop_session(&session_id)
        .await
        .map_err(StopError::Operator)?;
    session.stop();

    if let Err(e) = publisher.set_session_active(false).await {
        tracing::error!("Failed to publish SessionActive=false: {e}");
    }

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::BrokerError;
    use crate::operator::{OperatorApi, OperatorError, StartResponse, StopResponse};
    use crate::session::{Rate, Session};
    use async_trait::async_trait;

    // ──────────────────────────────────────────────────────────────────────────
    // Test helpers
    // ──────────────────────────────────────────────────────────────────────────

    fn make_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    fn make_start_response() -> StartResponse {
        StartResponse {
            session_id: "sess-1".to_string(),
            status: "active".to_string(),
            rate: make_rate(),
        }
    }

    fn make_stop_response() -> StopResponse {
        StopResponse {
            session_id: "sess-1".to_string(),
            status: "completed".to_string(),
            duration_seconds: 3600,
            total_amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Mock operator
    // ──────────────────────────────────────────────────────────────────────────

    #[derive(Default)]
    struct MockOperator {
        /// Preset response for `start_session`. Defaults to Ok(make_start_response()).
        start_response: Option<Result<StartResponse, OperatorError>>,
        /// Preset response for `stop_session`. Defaults to Ok(make_stop_response()).
        stop_response: Option<Result<StopResponse, OperatorError>>,
        start_calls: std::sync::Mutex<usize>,
        stop_calls: std::sync::Mutex<usize>,
        /// Captured (vehicle_id, zone_id) arguments from start_session calls.
        start_args: std::sync::Mutex<Vec<(String, String)>>,
        /// Captured session_id arguments from stop_session calls.
        stop_args: std::sync::Mutex<Vec<String>>,
    }

    impl MockOperator {
        fn start_call_count(&self) -> usize {
            *self.start_calls.lock().unwrap()
        }
        fn stop_call_count(&self) -> usize {
            *self.stop_calls.lock().unwrap()
        }
        /// Returns the (vehicle_id, zone_id) from the last start_session call.
        fn last_start_args(&self) -> Option<(String, String)> {
            self.start_args.lock().unwrap().last().cloned()
        }
        /// Returns the session_id from the last stop_session call.
        fn last_stop_session_id(&self) -> Option<String> {
            self.stop_args.lock().unwrap().last().cloned()
        }
    }

    #[async_trait]
    impl OperatorApi for MockOperator {
        async fn start_session(
            &self,
            vehicle_id: &str,
            zone_id: &str,
        ) -> Result<StartResponse, OperatorError> {
            *self.start_calls.lock().unwrap() += 1;
            self.start_args
                .lock()
                .unwrap()
                .push((vehicle_id.to_string(), zone_id.to_string()));
            self.start_response
                .clone()
                .unwrap_or_else(|| Ok(make_start_response()))
        }

        async fn stop_session(
            &self,
            session_id: &str,
        ) -> Result<StopResponse, OperatorError> {
            *self.stop_calls.lock().unwrap() += 1;
            self.stop_args
                .lock()
                .unwrap()
                .push(session_id.to_string());
            self.stop_response
                .clone()
                .unwrap_or_else(|| Ok(make_stop_response()))
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Mock publisher
    // ──────────────────────────────────────────────────────────────────────────

    #[derive(Default)]
    struct MockPublisher {
        calls: std::sync::Mutex<Vec<bool>>,
        should_fail: bool,
    }

    impl MockPublisher {
        fn last_call(&self) -> Option<bool> {
            self.calls.lock().unwrap().last().copied()
        }
    }

    #[async_trait]
    impl SessionPublisher for MockPublisher {
        async fn set_session_active(&self, active: bool) -> Result<(), BrokerError> {
            self.calls.lock().unwrap().push(active);
            if self.should_fail {
                Err(BrokerError::Unavailable("mock failure".into()))
            } else {
                Ok(())
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Tests: task 1.4 — event processing and gRPC handler tests
    // ──────────────────────────────────────────────────────────────────────────

    /// TS-08-2: StartSession RPC returns session_id, status, and rate.
    ///
    /// Verifies: 08-REQ-1.2
    #[tokio::test]
    async fn test_start_session_rpc() {
        let mut session = Session::new();
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        let result = manual_start("zone-a", &mut session, &op, &pub_, "DEMO-VIN-001").await;

        assert!(result.is_ok(), "manual_start must succeed");
        let resp = result.unwrap();
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");
        assert!(session.is_active());
    }

    /// TS-08-3: StopSession RPC returns duration and total_amount.
    ///
    /// Verifies: 08-REQ-1.3
    #[tokio::test]
    async fn test_stop_session_rpc() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-a".into(), 1_700_000_000, make_rate());
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        let result = manual_stop(&mut session, &op, &pub_).await;

        assert!(result.is_ok(), "manual_stop must succeed");
        let resp = result.unwrap();
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert!((resp.total_amount - 2.50).abs() < f64::EPSILON);
        assert!(!session.is_active());
    }

    /// TS-08-11: Lock event (IsLocked=true) triggers session start.
    ///
    /// Verifies: 08-REQ-3.3
    #[tokio::test]
    async fn test_lock_event_starts_session() {
        let mut session = Session::new();
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        process_lock_event(true, &mut session, &op, &pub_, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        assert!(session.is_active(), "session must be active after lock");
        assert_eq!(op.start_call_count(), 1, "operator start must be called once");
        assert_eq!(
            op.last_start_args(),
            Some(("DEMO-VIN-001".to_string(), "zone-demo-1".to_string())),
            "operator start must be called with correct vehicle_id and zone_id"
        );
        assert_eq!(
            pub_.last_call(),
            Some(true),
            "SessionActive must be set to true"
        );
    }

    /// TS-08-12: Unlock event (IsLocked=false) triggers session stop.
    ///
    /// Verifies: 08-REQ-3.4
    #[tokio::test]
    async fn test_unlock_event_stops_session() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-demo-1".into(), 1_700_000_000, make_rate());
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        process_lock_event(false, &mut session, &op, &pub_, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        assert!(!session.is_active(), "session must be inactive after unlock");
        assert_eq!(op.stop_call_count(), 1, "operator stop must be called once");
        assert_eq!(
            op.last_stop_session_id(),
            Some("sess-1".to_string()),
            "operator stop must be called with session_id 'sess-1'"
        );
        assert_eq!(
            pub_.last_call(),
            Some(false),
            "SessionActive must be set to false"
        );
    }

    /// TS-08-13: Vehicle.Parking.SessionActive is set to true on session start.
    ///
    /// Verifies: 08-REQ-4.1
    #[tokio::test]
    async fn test_session_active_set_true() {
        let mut session = Session::new();
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        process_lock_event(true, &mut session, &op, &pub_, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        assert_eq!(
            pub_.last_call(),
            Some(true),
            "SessionActive must be published as true"
        );
    }

    /// TS-08-14: Vehicle.Parking.SessionActive is set to false on session stop.
    ///
    /// Verifies: 08-REQ-4.2
    #[tokio::test]
    async fn test_session_active_set_false() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-demo-1".into(), 1_700_000_000, make_rate());
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        process_lock_event(false, &mut session, &op, &pub_, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        assert_eq!(
            pub_.last_call(),
            Some(false),
            "SessionActive must be published as false"
        );
    }

    /// TS-08-16: Manual StartSession works regardless of lock state.
    ///
    /// Verifies: 08-REQ-5.1
    #[tokio::test]
    async fn test_manual_start_override() {
        let mut session = Session::new();
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        // Vehicle is unlocked but manual start should work anyway.
        let result = manual_start("zone-manual", &mut session, &op, &pub_, "DEMO-VIN-001").await;

        assert!(result.is_ok(), "manual_start must succeed regardless of lock state");
        assert!(!result.unwrap().session_id.is_empty());
        assert!(session.is_active());
    }

    /// TS-08-17: Manual StopSession works regardless of lock state.
    ///
    /// Verifies: 08-REQ-5.2
    #[tokio::test]
    async fn test_manual_stop_override() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-a".into(), 1_700_000_000, make_rate());
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        // Vehicle is locked but manual stop should work anyway.
        let result = manual_stop(&mut session, &op, &pub_).await;

        assert!(result.is_ok(), "manual_stop must succeed regardless of lock state");
        assert!(!session.is_active());
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Tests: task 1.5 — edge case and override tests
    // ──────────────────────────────────────────────────────────────────────────

    /// TS-08-E1: StartSession returns AlreadyActive when a session is active.
    ///
    /// Verifies: 08-REQ-1.E1
    #[tokio::test]
    async fn test_start_session_already_active() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-a".into(), 1_700_000_000, make_rate());
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        let result = manual_start("zone-b", &mut session, &op, &pub_, "DEMO-VIN-001").await;

        assert!(result.is_err(), "expected error when session already active");
        assert!(
            matches!(result.unwrap_err(), StartError::AlreadyActive { .. }),
            "expected StartError::AlreadyActive"
        );
    }

    /// TS-08-E2: StopSession returns NotActive when no session exists.
    ///
    /// Verifies: 08-REQ-1.E2
    #[tokio::test]
    async fn test_stop_session_no_active() {
        let mut session = Session::new();
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        let result = manual_stop(&mut session, &op, &pub_).await;

        assert!(result.is_err(), "expected error when no session is active");
        assert!(
            matches!(result.unwrap_err(), StopError::NotActive),
            "expected StopError::NotActive"
        );
    }

    /// TS-08-E6: Lock event while session active is a no-op.
    ///
    /// Verifies: 08-REQ-3.E1
    #[tokio::test]
    async fn test_lock_event_noop_when_active() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-a".into(), 1_700_000_000, make_rate());
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        process_lock_event(true, &mut session, &op, &pub_, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        assert_eq!(op.start_call_count(), 0, "operator must not be called");
        assert!(session.is_active(), "session must remain active");
        assert_eq!(
            session.status().map(|s| s.session_id.as_str()),
            Some("sess-1"),
            "session_id must be unchanged"
        );
    }

    /// TS-08-E7: Unlock event without active session is a no-op.
    ///
    /// Verifies: 08-REQ-3.E2
    #[tokio::test]
    async fn test_unlock_event_noop_when_inactive() {
        let mut session = Session::new();
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        process_lock_event(false, &mut session, &op, &pub_, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();

        assert_eq!(op.stop_call_count(), 0, "operator must not be called");
        assert!(!session.is_active());
    }

    /// TS-08-E9: Failure to publish SessionActive is logged; operation continues.
    ///
    /// Verifies: 08-REQ-4.E1
    #[tokio::test]
    async fn test_session_active_publish_failure() {
        let mut session = Session::new();
        let op = MockOperator::default();
        let pub_ = MockPublisher {
            should_fail: true,
            ..Default::default()
        };

        // Must not panic — error is tolerated.
        let _ = process_lock_event(
            true,
            &mut session,
            &op,
            &pub_,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // In-memory state must still reflect the successful operator call.
        assert!(
            session.is_active(),
            "session state must be active despite publisher failure"
        );

        // Verify the service continues processing subsequent events (should not panic).
        let _ = process_lock_event(
            false,
            &mut session,
            &op,
            &pub_,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        assert!(
            !session.is_active(),
            "session must be inactive after subsequent unlock event"
        );
    }

    /// TS-08-E11: After a manual StopSession, the next lock event resumes
    /// autonomous session management.
    ///
    /// Verifies: 08-REQ-5.3, 08-REQ-5.E1
    #[tokio::test]
    async fn test_override_resumes_autonomous() {
        let mut session = Session::new();
        let op = MockOperator::default();
        let pub_ = MockPublisher::default();

        // Manual start → manual stop → autonomous lock event must create new session.
        manual_start("zone-a", &mut session, &op, &pub_, "DEMO-VIN-001")
            .await
            .unwrap();
        assert!(session.is_active());

        manual_stop(&mut session, &op, &pub_).await.unwrap();
        assert!(!session.is_active());

        // Autonomous behaviour resumes on next lock.
        process_lock_event(true, &mut session, &op, &pub_, "DEMO-VIN-001", "zone-demo-1")
            .await
            .unwrap();
        assert!(session.is_active(), "session must start autonomously");
        assert_eq!(
            op.start_call_count(),
            2,
            "operator start must have been called twice (manual + autonomous)"
        );
    }
}
