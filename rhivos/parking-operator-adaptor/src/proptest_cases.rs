//! Property-based tests for PARKING_OPERATOR_ADAPTOR correctness properties.
//!
//! All tests are `#[ignore]` — run explicitly with:
//!
//!     cargo test -p parking-operator-adaptor -- --ignored
//!
//! Properties correspond to Properties 1–6 in design.md.

use std::cell::RefCell;

use proptest::prelude::*;

use crate::broker::BrokerError;
use crate::event_loop::{manual_start, manual_stop, process_lock_event, EventError};
use crate::operator::{OperatorApi, OperatorError, RateResponse, StartResponse, StopResponse};
use crate::session::{Rate, Session};

// ── Shared mock types ─────────────────────────────────────────────────────────

struct MockOperator {
    fail: bool,
    start_count: RefCell<usize>,
    stop_count: RefCell<usize>,
}

impl MockOperator {
    fn success() -> Self {
        Self { fail: false, start_count: RefCell::new(0), stop_count: RefCell::new(0) }
    }

    fn failing() -> Self {
        Self { fail: true, ..Self::success() }
    }
}

impl OperatorApi for MockOperator {
    async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        *self.start_count.borrow_mut() += 1;
        if self.fail {
            return Err(OperatorError::RetriesExhausted);
        }
        Ok(StartResponse {
            session_id: "prop-sess".to_owned(),
            status: "active".to_owned(),
            rate: RateResponse {
                rate_type: "per_hour".to_owned(),
                amount: 1.0,
                currency: "EUR".to_owned(),
            },
        })
    }

    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        *self.stop_count.borrow_mut() += 1;
        if self.fail {
            return Err(OperatorError::RetriesExhausted);
        }
        Ok(StopResponse {
            session_id: session_id.to_owned(),
            status: "completed".to_owned(),
            duration_seconds: 60,
            total_amount: 1.0,
            currency: "EUR".to_owned(),
        })
    }
}

struct MockPublisher {
    fail: bool,
    last_value: RefCell<Option<bool>>,
}

impl MockPublisher {
    fn success() -> Self {
        Self { fail: false, last_value: RefCell::new(None) }
    }

    fn failing() -> Self {
        Self { fail: true, last_value: RefCell::new(None) }
    }
}

impl crate::broker::SessionPublisher for MockPublisher {
    async fn set_session_active(&self, active: bool) -> Result<(), BrokerError> {
        *self.last_value.borrow_mut() = Some(active);
        if self.fail {
            Err(BrokerError::PublishFailed("injected".to_owned()))
        } else {
            Ok(())
        }
    }
}

fn make_rate() -> Rate {
    Rate { rate_type: "per_hour".to_owned(), amount: 1.0, currency: "EUR".to_owned() }
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

// ── TS-08-P1: Session State Consistency ──────────────────────────────────────

proptest! {
    #[test]
    #[ignore]
    fn proptest_session_state_consistency(
        // ops: true = Start, false = Stop; outcomes: true = Ok, false = Err
        ops in proptest::collection::vec(proptest::bool::ANY, 0..10),
        outcomes in proptest::collection::vec(proptest::bool::ANY, 0..10),
    ) {
        let runtime = rt();
        let mut session = Session::new();
        let mut last_successful_is_start: Option<bool> = None;

        for (is_start, succeeds) in ops.iter().zip(outcomes.iter()) {
            let operator = if *succeeds { MockOperator::success() } else { MockOperator::failing() };
            let publisher = MockPublisher::success();

            let result = runtime.block_on(async {
                if *is_start && !session.is_active() {
                    manual_start("zone", &mut session, &operator, &publisher, "VIN")
                        .await
                        .map(|_| true)
                        .or(Ok::<bool, EventError>(false))
                } else if !is_start && session.is_active() {
                    manual_stop(&mut session, &operator, &publisher)
                        .await
                        .map(|_| false)
                        .or(Ok::<bool, EventError>(true))
                } else {
                    Ok(session.is_active())
                }
            });

            if *succeeds {
                last_successful_is_start = Some(*is_start && !matches!(result, Ok(false)));
            }
        }

        // Final invariant: active matches last successful operation.
        match last_successful_is_start {
            Some(true) => prop_assert!(session.is_active()),
            Some(false) => prop_assert!(!session.is_active()),
            None => prop_assert!(!session.is_active()), // no successful op → inactive
        }
    }
}

// ── TS-08-P2: Idempotent Lock Events ─────────────────────────────────────────

proptest! {
    #[test]
    #[ignore]
    fn proptest_idempotent_lock_events(
        n in 1usize..10,
        m in 1usize..10,
    ) {
        let runtime = rt();
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        // N consecutive lock events.
        for _ in 0..n {
            let _ = runtime.block_on(process_lock_event(
                true, &mut session, &operator, &publisher, "VIN", "zone",
            ));
        }
        prop_assert_eq!(*operator.start_count.borrow(), 1, "start called exactly once");

        // M consecutive unlock events.
        for _ in 0..m {
            let _ = runtime.block_on(process_lock_event(
                false, &mut session, &operator, &publisher, "VIN", "zone",
            ));
        }
        prop_assert_eq!(*operator.stop_count.borrow(), 1, "stop called exactly once");
    }
}

// ── TS-08-P3: Override Non-Persistence ───────────────────────────────────────

proptest! {
    #[test]
    #[ignore]
    fn proptest_override_non_persistence(manual_start_first in proptest::bool::ANY) {
        let runtime = rt();
        let operator = MockOperator::success();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        if manual_start_first {
            let _ = runtime.block_on(
                manual_start("zone", &mut session, &operator, &publisher, "VIN")
            );
            let _ = runtime.block_on(manual_stop(&mut session, &operator, &publisher));
        } else {
            // Pre-start then manual stop.
            session.start("s1".to_owned(), "z".to_owned(), 0, make_rate());
            let _ = runtime.block_on(manual_stop(&mut session, &operator, &publisher));
        }

        // Next lock event should autonomously start a new session.
        runtime.block_on(process_lock_event(
            true, &mut session, &operator, &publisher, "VIN", "zone",
        )).expect("autonomous start must succeed after manual override");

        prop_assert!(session.is_active(), "session should be active after autonomous lock");
    }
}

// ── TS-08-P4: Retry Exhaustion Safety ────────────────────────────────────────

proptest! {
    #[test]
    #[ignore]
    fn proptest_retry_exhaustion_safety(_seed in proptest::bool::ANY) {
        let runtime = rt();
        let operator = MockOperator::failing();
        let publisher = MockPublisher::success();
        let mut session = Session::new();

        // Failed start must not change session state.
        let _ = runtime.block_on(process_lock_event(
            true, &mut session, &operator, &publisher, "VIN", "zone",
        ));

        prop_assert!(!session.is_active(), "session must remain inactive after failed start");
    }
}

// ── TS-08-P5: SessionActive Signal Consistency ───────────────────────────────

proptest! {
    #[test]
    #[ignore]
    fn proptest_session_active_consistency(
        ops in proptest::collection::vec(proptest::bool::ANY, 1..8),
    ) {
        let runtime = rt();
        let publisher = MockPublisher::success();
        let operator = MockOperator::success();
        let mut session = Session::new();

        for is_lock in &ops {
            let _ = runtime.block_on(process_lock_event(
                *is_lock, &mut session, &operator, &publisher, "VIN", "zone",
            ));
        }

        // Last set_session_active call must match session.is_active().
        let last_signal_opt: Option<bool> = *publisher.last_value.borrow();
        if let Some(last_signal) = last_signal_opt {
            prop_assert_eq!(
                last_signal,
                session.is_active(),
                "SessionActive signal must match in-memory session state"
            );
        }
    }
}

// ── TS-08-P6: Sequential Event Processing ────────────────────────────────────

proptest! {
    #[test]
    #[ignore]
    fn proptest_sequential_event_processing(
        events in proptest::collection::vec(proptest::bool::ANY, 2..6),
    ) {
        // Verify that processing events sequentially yields a deterministic final state.
        let runtime = rt();

        let process = |evts: &[bool]| -> bool {
            let operator = MockOperator::success();
            let publisher = MockPublisher::success();
            let mut session = Session::new();
            for &is_lock in evts {
                let _ = runtime.block_on(process_lock_event(
                    is_lock, &mut session, &operator, &publisher, "VIN", "zone",
                ));
            }
            session.is_active()
        };

        let result_a = process(&events);
        let result_b = process(&events);

        prop_assert_eq!(result_a, result_b, "sequential processing must be deterministic");
    }
}
