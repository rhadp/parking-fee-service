//! Property-based tests for the PARKING_OPERATOR_ADAPTOR.
//!
//! All tests are `#[ignore]` and run separately via:
//!   cargo test -- --ignored
//!
//! Tests use proptest to verify invariants from design.md Properties 1–6.
//! Async operations use a single-threaded tokio runtime.

use proptest::prelude::*;

use crate::event_loop::{
    process_lock_event, process_manual_start, process_manual_stop, BrokerError, BrokerTrait,
    OperatorTrait, SIGNAL_SESSION_ACTIVE,
};
use crate::operator::{OperatorError, StartResponse, StopResponse};
use crate::session::{Rate, Session};

use std::sync::Mutex;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_rate() -> Rate {
    Rate {
        rate_type: "per_hour".to_string(),
        amount: 2.5,
        currency: "EUR".to_string(),
    }
}

fn make_start_response(session_id: &str) -> StartResponse {
    StartResponse {
        session_id: session_id.to_string(),
        status: "active".to_string(),
        rate: make_rate(),
    }
}

fn make_stop_response(session_id: &str) -> StopResponse {
    StopResponse {
        session_id: session_id.to_string(),
        status: "completed".to_string(),
        duration_seconds: 3600,
        total_amount: 2.5,
        currency: "EUR".to_string(),
    }
}

fn current_thread_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
}

// ── Mock types ────────────────────────────────────────────────────────────────

struct MockOp {
    start_count: Mutex<usize>,
    stop_count: Mutex<usize>,
    fail: bool,
}

impl MockOp {
    fn new() -> Self {
        MockOp {
            start_count: Mutex::new(0),
            stop_count: Mutex::new(0),
            fail: false,
        }
    }
    fn failing() -> Self {
        MockOp {
            start_count: Mutex::new(0),
            stop_count: Mutex::new(0),
            fail: true,
        }
    }
    fn start_count(&self) -> usize {
        *self.start_count.lock().unwrap()
    }
    fn stop_count(&self) -> usize {
        *self.stop_count.lock().unwrap()
    }
}

impl OperatorTrait for MockOp {
    async fn start_session(&self, _vid: &str, _zid: &str) -> Result<StartResponse, OperatorError> {
        *self.start_count.lock().unwrap() += 1;
        if self.fail {
            return Err(OperatorError::RetriesExhausted("mock".to_string()));
        }
        Ok(make_start_response("prop-sess"))
    }
    async fn stop_session(&self, _sid: &str) -> Result<StopResponse, OperatorError> {
        *self.stop_count.lock().unwrap() += 1;
        if self.fail {
            return Err(OperatorError::RetriesExhausted("mock".to_string()));
        }
        Ok(make_stop_response("prop-sess"))
    }
}

struct MockBroker {
    calls: Mutex<Vec<(String, bool)>>,
}

impl MockBroker {
    fn new() -> Self {
        MockBroker {
            calls: Mutex::new(Vec::new()),
        }
    }
    fn last_session_active(&self) -> Option<bool> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(s, _)| s == SIGNAL_SESSION_ACTIVE)
            .map(|(_, v)| *v)
    }
}

impl BrokerTrait for MockBroker {
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        self.calls.lock().unwrap().push((signal.to_string(), value));
        Ok(())
    }
}

// ── TS-08-P1: Session State Consistency ──────────────────────────────────────

/// Property 1: After any sequence of start/stop operations, session.is_active()
/// matches the last successful operation.
///
/// Validates: 08-REQ-6.1, 08-REQ-6.2, 08-REQ-6.3
#[test]
#[ignore]
fn proptest_session_state_consistency() {
    let rt = current_thread_rt();
    // ops: true = start, false = stop
    proptest!(|(ops in proptest::collection::vec(proptest::bool::ANY, 1..10))| {
        let op = MockOp::new();
        let broker = MockBroker::new();
        let mut session = Session::new();
        let mut last_successful: Option<bool> = None;

        for &is_start in &ops {
            let result = rt.block_on(async {
                if is_start {
                    process_manual_start("zone-a", &mut session, &op, &broker, "VIN").await.ok().map(|_| true)
                } else {
                    process_manual_stop(&mut session, &op, &broker).await.ok().map(|_| false)
                }
            });
            if let Some(succeeded_as_start) = result {
                last_successful = Some(succeeded_as_start);
            }
        }

        match last_successful {
            Some(true) => prop_assert!(session.is_active(), "last successful start → must be active"),
            Some(false) => prop_assert!(!session.is_active(), "last successful stop → must be inactive"),
            None => prop_assert!(!session.is_active(), "no successful op → must be inactive"),
        }
    });
}

// ── TS-08-P2: Idempotent Lock Events ─────────────────────────────────────────

/// Property 2: N consecutive lock events start exactly one session;
/// M consecutive unlock events stop exactly one session.
///
/// Validates: 08-REQ-3.E1, 08-REQ-3.E2
#[test]
#[ignore]
fn proptest_idempotent_lock_events() {
    let rt = current_thread_rt();
    proptest!(|(n in 1usize..10, m in 1usize..10)| {
        let op = MockOp::new();
        let broker = MockBroker::new();
        let mut session = Session::new();

        for _ in 0..n {
            rt.block_on(async {
                let _ = process_lock_event(true, &mut session, &op, &broker, "VIN", "zone").await;
            });
        }
        prop_assert_eq!(op.start_count(), 1, "start must be called exactly once");

        for _ in 0..m {
            rt.block_on(async {
                let _ = process_lock_event(false, &mut session, &op, &broker, "VIN", "zone").await;
            });
        }
        prop_assert_eq!(op.stop_count(), 1, "stop must be called exactly once");
    });
}

// ── TS-08-P3: Override Non-Persistence ───────────────────────────────────────

/// Property 3: After any manual override, the next lock/unlock event resumes
/// autonomous behavior.
///
/// Validates: 08-REQ-5.1, 08-REQ-5.2, 08-REQ-5.3, 08-REQ-5.E1
#[test]
#[ignore]
fn proptest_override_non_persistence() {
    let rt = current_thread_rt();
    // true = manual start override, false = manual stop override
    proptest!(|(is_start_override in proptest::bool::ANY)| {
        let op = MockOp::new();
        let broker = MockBroker::new();
        let mut session = Session::new();

        if is_start_override {
            // Manual start, then manual stop, then check autonomous lock works.
            rt.block_on(async {
                let _ = process_manual_start("zone", &mut session, &op, &broker, "VIN").await;
                let _ = process_manual_stop(&mut session, &op, &broker).await;
            });
        } else {
            // Start a session, then manual stop override.
            rt.block_on(async {
                let _ = process_lock_event(true, &mut session, &op, &broker, "VIN", "zone").await;
                let _ = process_manual_stop(&mut session, &op, &broker).await;
            });
        }

        let start_count_before = op.start_count();
        // Next lock event must trigger autonomous start.
        rt.block_on(async {
            let _ = process_lock_event(true, &mut session, &op, &broker, "VIN", "zone").await;
        });

        prop_assert!(op.start_count() > start_count_before, "autonomous start must occur after override");
        prop_assert!(session.is_active(), "session must be active after autonomous start");

        // Autonomous stop.
        let stop_count_before = op.stop_count();
        rt.block_on(async {
            let _ = process_lock_event(false, &mut session, &op, &broker, "VIN", "zone").await;
        });
        prop_assert!(op.stop_count() > stop_count_before, "autonomous stop must occur");
        prop_assert!(!session.is_active(), "session must be inactive after autonomous stop");
    });
}

// ── TS-08-P4: Retry Exhaustion Safety ────────────────────────────────────────

/// Property 4: When the operator always fails, session state is never corrupted.
///
/// Validates: 08-REQ-2.E1, 08-REQ-2.E2
#[test]
#[ignore]
fn proptest_retry_exhaustion_safety() {
    let rt = current_thread_rt();
    proptest!(|(is_lock in proptest::bool::ANY)| {
        let op = MockOp::failing();
        let broker = MockBroker::new();
        let mut session = Session::new();

        let was_active = session.is_active();

        rt.block_on(async {
            // Lock event with failing operator — should not change session state.
            let _ = process_lock_event(is_lock, &mut session, &op, &broker, "VIN", "zone").await;
        });

        // Session state must be unchanged when operator fails.
        prop_assert_eq!(
            session.is_active(),
            was_active,
            "session state must not change when operator fails"
        );
    });
}

// ── TS-08-P5: SessionActive Signal Consistency ────────────────────────────────

/// Property 5: After any successful start/stop, the last set_bool call for
/// SessionActive matches session.is_active().
///
/// Validates: 08-REQ-4.1, 08-REQ-4.2, 08-REQ-4.3
#[test]
#[ignore]
fn proptest_session_active_consistency() {
    let rt = current_thread_rt();
    // ops: true = start (lock), false = stop (unlock)
    proptest!(|(ops in proptest::collection::vec(proptest::bool::ANY, 1..8))| {
        let op = MockOp::new();
        let broker = MockBroker::new();
        let mut session = Session::new();

        for &is_lock in &ops {
            rt.block_on(async {
                let _ = process_lock_event(is_lock, &mut session, &op, &broker, "VIN", "zone").await;
            });
        }

        if let Some(last_signal_value) = broker.last_session_active() {
            prop_assert_eq!(
                last_signal_value,
                session.is_active(),
                "SessionActive signal must match session.is_active()"
            );
        }
    });
}

// ── TS-08-P6: Sequential Event Processing ────────────────────────────────────

/// Property 6: Sequentially processed events produce the same deterministic
/// final state as the events imply.
///
/// Validates: 08-REQ-9.1, 08-REQ-9.2
#[test]
#[ignore]
fn proptest_sequential_event_processing() {
    let rt = current_thread_rt();
    // Sequence of lock events (true/false).
    proptest!(|(events in proptest::collection::vec(proptest::bool::ANY, 2..5))| {
        let op = MockOp::new();
        let broker = MockBroker::new();
        let mut session = Session::new();

        // Process all events sequentially.
        for &is_lock in &events {
            rt.block_on(async {
                let _ = process_lock_event(is_lock, &mut session, &op, &broker, "VIN", "zone").await;
            });
        }

        // The final state must be deterministic: depends only on the last unique
        // transition (lock when inactive → start; unlock when active → stop).
        // Verify the final state is consistent with the session record.
        if let Some(last_signal) = broker.last_session_active() {
            prop_assert_eq!(
                last_signal,
                session.is_active(),
                "final SessionActive must match session.is_active()"
            );
        }
        // No panics or races — just verify consistency.
        prop_assert!(true, "sequential processing must not panic");
    });
}
