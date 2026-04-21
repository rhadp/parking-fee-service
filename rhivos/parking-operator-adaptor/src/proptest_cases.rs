//! Property-based tests for parking-operator-adaptor.
//!
//! Run with: `cargo test -p parking-operator-adaptor -- --include-ignored proptest`
//!
//! All tests in this module are marked `#[ignore]` to keep the default test
//! run fast.  The proptest runner catches panics, so `assert!` / `assert_eq!`
//! are used in place of the `prop_assert!` family to work correctly inside
//! `tokio::runtime::Runtime::block_on` closures.

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use proptest::prelude::*;

    use crate::broker::{BrokerError, SessionPublisher};
    use crate::event_loop::{manual_start, manual_stop, process_lock_event};
    use crate::operator::{OperatorApi, OperatorError, StartResponse, StopResponse};
    use crate::session::{Rate, Session};

    // ── Shared helpers ────────────────────────────────────────────────────────

    fn make_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    fn make_start_response() -> StartResponse {
        StartResponse {
            session_id: "sess-prop".to_string(),
            status: "active".to_string(),
            rate: make_rate(),
        }
    }

    fn make_stop_response() -> StopResponse {
        StopResponse {
            session_id: "sess-prop".to_string(),
            status: "completed".to_string(),
            duration_seconds: 60,
            total_amount: 1.00,
            currency: "EUR".to_string(),
        }
    }

    fn new_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().expect("failed to create Tokio runtime")
    }

    // ── Mock operator ─────────────────────────────────────────────────────────

    #[derive(Default)]
    struct MockOperator {
        always_fail: bool,
        start_calls: std::sync::Mutex<usize>,
        stop_calls: std::sync::Mutex<usize>,
    }

    impl MockOperator {
        fn start_call_count(&self) -> usize {
            *self.start_calls.lock().unwrap()
        }
        fn stop_call_count(&self) -> usize {
            *self.stop_calls.lock().unwrap()
        }
        fn reset_counts(&self) {
            *self.start_calls.lock().unwrap() = 0;
            *self.stop_calls.lock().unwrap() = 0;
        }
    }

    #[async_trait]
    impl OperatorApi for MockOperator {
        async fn start_session(
            &self,
            _vehicle_id: &str,
            _zone_id: &str,
        ) -> Result<StartResponse, OperatorError> {
            *self.start_calls.lock().unwrap() += 1;
            if self.always_fail {
                Err(OperatorError::Unavailable("forced failure".into()))
            } else {
                Ok(make_start_response())
            }
        }

        async fn stop_session(
            &self,
            _session_id: &str,
        ) -> Result<StopResponse, OperatorError> {
            *self.stop_calls.lock().unwrap() += 1;
            if self.always_fail {
                Err(OperatorError::Unavailable("forced failure".into()))
            } else {
                Ok(make_stop_response())
            }
        }
    }

    // ── Mock publisher ────────────────────────────────────────────────────────

    #[derive(Default)]
    struct MockPublisher {
        calls: std::sync::Mutex<Vec<bool>>,
    }

    impl MockPublisher {
        fn last_value(&self) -> Option<bool> {
            self.calls.lock().unwrap().last().copied()
        }
    }

    #[async_trait]
    impl SessionPublisher for MockPublisher {
        async fn set_session_active(&self, active: bool) -> Result<(), BrokerError> {
            self.calls.lock().unwrap().push(active);
            Ok(())
        }
    }

    // ── TS-08-P1: Session State Consistency ───────────────────────────────────

    /// Property 1: after any sequence of start/stop operations, session state
    /// matches the outcome of the last successful operation.
    ///
    /// Verifies: 08-REQ-6.1, 08-REQ-6.2, 08-REQ-6.3
    #[test]
    #[ignore = "proptest - run with --include-ignored proptest"]
    fn proptest_session_state_consistency() {
        // ops: 0 = Start attempt, 1 = Stop attempt
        proptest!(|(ops in proptest::collection::vec(0u8..2u8, 1..=10))| {
            new_runtime().block_on(async {
                let mut session = Session::new();
                let op = MockOperator::default();
                let pub_ = MockPublisher::default();
                let mut last_was_start: Option<bool> = None;

                for &action in &ops {
                    if action == 0 && !session.is_active() {
                        if manual_start("zone", &mut session, &op, &pub_, "VID").await.is_ok() {
                            last_was_start = Some(true);
                        }
                    } else if action == 1 && session.is_active() {
                        if manual_stop(&mut session, &op, &pub_).await.is_ok() {
                            last_was_start = Some(false);
                        }
                    }
                }

                match last_was_start {
                    Some(true) => assert!(session.is_active()),
                    Some(false) | None => assert!(!session.is_active()),
                }
            });
        });
    }

    // ── TS-08-P2: Idempotent Lock Events ─────────────────────────────────────

    /// Property 2: N consecutive lock events trigger exactly one operator start;
    /// M consecutive unlock events trigger exactly one operator stop.
    ///
    /// Verifies: 08-REQ-3.E1, 08-REQ-3.E2
    #[test]
    #[ignore = "proptest - run with --include-ignored proptest"]
    fn proptest_idempotent_lock_events() {
        proptest!(|(n in 1usize..=5, m in 1usize..=5)| {
            new_runtime().block_on(async {
                let mut session = Session::new();
                let op = MockOperator::default();
                let pub_ = MockPublisher::default();

                for _ in 0..n {
                    let _ = process_lock_event(
                        true, &mut session, &op, &pub_, "VID", "zone",
                    ).await;
                }
                assert_eq!(op.start_call_count(), 1, "start called exactly once for N lock events");

                for _ in 0..m {
                    let _ = process_lock_event(
                        false, &mut session, &op, &pub_, "VID", "zone",
                    ).await;
                }
                assert_eq!(op.stop_call_count(), 1, "stop called exactly once for M unlock events");
            });
        });
    }

    // ── TS-08-P3: Override Non-Persistence ───────────────────────────────────

    /// Property 3: after any manual override, the next lock/unlock cycle
    /// resumes autonomous session management.
    ///
    /// Verifies: 08-REQ-5.1, 08-REQ-5.2, 08-REQ-5.3, 08-REQ-5.E1
    #[test]
    #[ignore = "proptest - run with --include-ignored proptest"]
    fn proptest_override_non_persistence() {
        // override_type: 0 = ManualStart then ManualStop, 1 = autonomous start then ManualStop
        proptest!(|(override_type in 0u8..2u8)| {
            new_runtime().block_on(async {
                let mut session = Session::new();
                let op = MockOperator::default();
                let pub_ = MockPublisher::default();

                if override_type == 0 {
                    // Manual start → manual stop
                    let _ = manual_start("zone", &mut session, &op, &pub_, "VID").await;
                    let _ = manual_stop(&mut session, &op, &pub_).await;
                } else {
                    // Autonomous start, then manual stop
                    let _ = process_lock_event(
                        true, &mut session, &op, &pub_, "VID", "zone",
                    ).await;
                    let _ = manual_stop(&mut session, &op, &pub_).await;
                }

                op.reset_counts();

                // Autonomous lock must start a new session.
                let _ = process_lock_event(
                    true, &mut session, &op, &pub_, "VID", "zone",
                ).await;
                assert!(session.is_active(), "session must be active after autonomous lock");
                assert_eq!(op.start_call_count(), 1, "operator start called once autonomously");

                // Autonomous unlock must stop the session.
                let _ = process_lock_event(
                    false, &mut session, &op, &pub_, "VID", "zone",
                ).await;
                assert!(!session.is_active(), "session must be inactive after autonomous unlock");
            });
        });
    }

    // ── TS-08-P4: Retry Exhaustion Safety ────────────────────────────────────

    /// Property 4: a failed REST call never corrupts in-memory session state.
    ///
    /// Verifies: 08-REQ-2.E1, 08-REQ-2.E2
    #[test]
    #[ignore = "proptest - run with --include-ignored proptest"]
    fn proptest_retry_exhaustion_safety() {
        proptest!(|(_dummy in 0u8..1u8)| {
            new_runtime().block_on(async {
                let mut session = Session::new();
                let op = MockOperator { always_fail: true, ..Default::default() };
                let pub_ = MockPublisher::default();

                let was_active_before = session.is_active();

                // A failing lock event must not change session state.
                let _ = process_lock_event(
                    true, &mut session, &op, &pub_, "VID", "zone",
                ).await;

                assert_eq!(
                    session.is_active(),
                    was_active_before,
                    "session state must be unchanged after operator failure"
                );
            });
        });
    }

    // ── TS-08-P5: SessionActive Signal Consistency ───────────────────────────

    /// Property 5: after each successful start/stop, the last published
    /// SessionActive value matches session.is_active().
    ///
    /// Verifies: 08-REQ-4.1, 08-REQ-4.2, 08-REQ-4.3
    #[test]
    #[ignore = "proptest - run with --include-ignored proptest"]
    fn proptest_session_active_consistency() {
        // ops: 0 = lock (start), 1 = unlock (stop)
        proptest!(|(ops in proptest::collection::vec(0u8..2u8, 1..=8))| {
            new_runtime().block_on(async {
                let mut session = Session::new();
                let op = MockOperator::default();
                let pub_ = MockPublisher::default();

                for &action in &ops {
                    let is_locked = action == 0;
                    let _ = process_lock_event(
                        is_locked, &mut session, &op, &pub_, "VID", "zone",
                    ).await;
                }

                // After all ops, the last published value must match session state.
                if let Some(last) = pub_.last_value() {
                    assert_eq!(
                        last,
                        session.is_active(),
                        "last published SessionActive must match session.is_active()"
                    );
                }
            });
        });
    }

    // ── TS-08-P6: Sequential Event Processing ────────────────────────────────

    /// Property 6: events processed sequentially produce a deterministic final
    /// state (no race conditions).
    ///
    /// Verifies: 08-REQ-9.1, 08-REQ-9.2, 08-REQ-9.E1
    #[test]
    #[ignore = "proptest - run with --include-ignored proptest"]
    fn proptest_sequential_event_processing() {
        // events: 0 = lock, 1 = unlock, 2 = manual_start, 3 = manual_stop
        proptest!(|(events in proptest::collection::vec(0u8..4u8, 2..=5))| {
            new_runtime().block_on(async {
                let mut session = Session::new();
                let op = MockOperator::default();
                let pub_ = MockPublisher::default();

                for &ev in &events {
                    match ev {
                        0 => {
                            let _ = process_lock_event(
                                true, &mut session, &op, &pub_, "VID", "zone",
                            ).await;
                        }
                        1 => {
                            let _ = process_lock_event(
                                false, &mut session, &op, &pub_, "VID", "zone",
                            ).await;
                        }
                        2 => {
                            let _ = manual_start("zone", &mut session, &op, &pub_, "VID").await;
                        }
                        _ => {
                            let _ = manual_stop(&mut session, &op, &pub_).await;
                        }
                    }
                }

                // Invariant: session.is_active() must match the broker signal.
                if let Some(last) = pub_.last_value() {
                    assert_eq!(
                        last,
                        session.is_active(),
                        "broker signal and in-memory state must agree"
                    );
                }
            });
        });
    }
}
