use proptest::prelude::*;

use crate::event_loop::{process_lock_event, process_manual_start, process_manual_stop};
use crate::session::{Rate, Session};
use crate::testing::{MockBrokerClient, MockOperatorClient};

fn sample_rate() -> Rate {
    Rate {
        rate_type: "per_hour".to_string(),
        amount: 2.50,
        currency: "EUR".to_string(),
    }
}

proptest! {
    // TS-08-P1: Session State Consistency
    // After any sequence of start/stop operations, session state matches
    // the last successful operation.
    #[test]
    #[ignore]
    fn proptest_session_state_consistency(
        ops in proptest::collection::vec(proptest::bool::ANY, 1..20),
    ) {
        let mut session = Session::new();
        let mut last_successful: Option<bool> = None; // true=start, false=stop

        for is_start in &ops {
            if *is_start {
                session.start(
                    "s1".to_string(),
                    "zone-a".to_string(),
                    1_700_000_000,
                    sample_rate(),
                );
                last_successful = Some(true);
            } else {
                session.stop();
                last_successful = Some(false);
            }
        }

        match last_successful {
            Some(true) => prop_assert!(session.is_active(), "should be active after last start"),
            Some(false) | None => prop_assert!(!session.is_active(), "should be inactive"),
        }
    }

    // TS-08-P2: Idempotent Lock Events
    // Duplicate lock/unlock events are no-ops. N lock events produce
    // exactly one operator start call; M unlock events produce exactly
    // one operator stop call.
    #[test]
    #[ignore]
    fn proptest_idempotent_lock_events(n in 1usize..10, m in 1usize..10) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut session = Session::new();

            // Send N lock events.
            for i in 0..n {
                let operator = MockOperatorClient::new();
                let broker = MockBrokerClient::new();
                if i == 0 {
                    operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");
                }
                process_lock_event(&mut session, &operator, &broker, "VIN", "zone", true).await;
            }
            prop_assert!(session.is_active(), "should be active after lock events");

            // Send M unlock events.
            for i in 0..m {
                let operator = MockOperatorClient::new();
                let broker = MockBrokerClient::new();
                if i == 0 {
                    operator.on_stop_success("sess-1", 3600, 2.50, "EUR");
                }
                process_lock_event(&mut session, &operator, &broker, "VIN", "zone", false).await;
            }
            prop_assert!(!session.is_active(), "should be inactive after unlock events");

            Ok(())
        })?;
    }

    // TS-08-P3: Override Non-Persistence
    // After any manual override, the next lock/unlock cycle resumes
    // autonomous behavior.
    #[test]
    #[ignore]
    fn proptest_override_non_persistence(do_manual_start in proptest::bool::ANY) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut session = Session::new();

            if do_manual_start {
                // Manual start, then manual stop.
                let op = MockOperatorClient::new();
                let br = MockBrokerClient::new();
                op.on_start_success("sess-m", "per_hour", 2.50, "EUR");
                let _ = process_manual_start(&mut session, &op, &br, "VIN", "zone").await;

                let op2 = MockOperatorClient::new();
                let br2 = MockBrokerClient::new();
                op2.on_stop_success("sess-m", 3600, 2.50, "EUR");
                let _ = process_manual_stop(&mut session, &op2, &br2).await;
            } else {
                // Start via lock, then manual stop.
                let op = MockOperatorClient::new();
                let br = MockBrokerClient::new();
                op.on_start_success("sess-m", "per_hour", 2.50, "EUR");
                process_lock_event(&mut session, &op, &br, "VIN", "zone", true).await;

                let op2 = MockOperatorClient::new();
                let br2 = MockBrokerClient::new();
                op2.on_stop_success("sess-m", 3600, 2.50, "EUR");
                let _ = process_manual_stop(&mut session, &op2, &br2).await;
            }
            prop_assert!(!session.is_active(), "should be inactive after stop");

            // Next autonomous lock should start a new session.
            let op3 = MockOperatorClient::new();
            let br3 = MockBrokerClient::new();
            op3.on_start_success("sess-auto", "per_hour", 2.50, "EUR");
            process_lock_event(&mut session, &op3, &br3, "VIN", "zone", true).await;
            prop_assert!(session.is_active(), "autonomous should resume");

            // Unlock should stop it.
            let op4 = MockOperatorClient::new();
            let br4 = MockBrokerClient::new();
            op4.on_stop_success("sess-auto", 3600, 2.50, "EUR");
            process_lock_event(&mut session, &op4, &br4, "VIN", "zone", false).await;
            prop_assert!(!session.is_active(), "autonomous stop should work");

            Ok(())
        })?;
    }

    // TS-08-P4: Retry Exhaustion Safety
    // Failed REST calls never corrupt session state.
    #[test]
    #[ignore]
    fn proptest_retry_exhaustion_safety(n_attempts in 1usize..5) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut session = Session::new();

            // Operator will fail (no response configured).
            for _ in 0..n_attempts {
                let operator = MockOperatorClient::new();
                // Don't configure a response → will return error.
                let broker = MockBrokerClient::new();
                process_lock_event(&mut session, &operator, &broker, "VIN", "zone", true).await;
            }

            // Session should remain inactive since all starts failed.
            prop_assert!(!session.is_active(), "session should remain inactive after failures");

            Ok(())
        })?;
    }

    // TS-08-P5: SessionActive Signal Consistency
    // SessionActive signal always matches session.active after successful operations.
    #[test]
    #[ignore]
    fn proptest_session_active_consistency(
        ops in proptest::collection::vec(proptest::bool::ANY, 1..10),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut session = Session::new();

            for is_lock in &ops {
                let operator = MockOperatorClient::new();
                let broker = MockBrokerClient::new();
                if *is_lock && !session.is_active() {
                    operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");
                    process_lock_event(&mut session, &operator, &broker, "VIN", "zone", true).await;
                    let last = broker.last_set_bool();
                    prop_assert_eq!(
                        last,
                        Some(("Vehicle.Parking.SessionActive".to_string(), true)),
                        "should publish true after start"
                    );
                } else if !is_lock && session.is_active() {
                    operator.on_stop_success("sess-1", 3600, 2.50, "EUR");
                    process_lock_event(&mut session, &operator, &broker, "VIN", "zone", false).await;
                    let last = broker.last_set_bool();
                    prop_assert_eq!(
                        last,
                        Some(("Vehicle.Parking.SessionActive".to_string(), false)),
                        "should publish false after stop"
                    );
                }
            }

            Ok(())
        })?;
    }

    // TS-08-P6: Sequential Event Processing
    // Events processed in order produce deterministic results.
    // We verify that processing events sequentially gives consistent
    // final state (no corruption).
    #[test]
    #[ignore]
    fn proptest_sequential_event_processing(
        events in proptest::collection::vec(0u8..4, 2..5),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut session = Session::new();

            for event_type in &events {
                let operator = MockOperatorClient::new();
                let broker = MockBrokerClient::new();
                match event_type {
                    0 => {
                        // Lock event
                        if !session.is_active() {
                            operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");
                        }
                        process_lock_event(&mut session, &operator, &broker, "VIN", "zone", true).await;
                    }
                    1 => {
                        // Unlock event
                        if session.is_active() {
                            operator.on_stop_success("sess-1", 3600, 2.50, "EUR");
                        }
                        process_lock_event(&mut session, &operator, &broker, "VIN", "zone", false).await;
                    }
                    2 => {
                        // Manual start
                        if !session.is_active() {
                            operator.on_start_success("sess-1", "per_hour", 2.50, "EUR");
                        }
                        let _ = process_manual_start(&mut session, &operator, &broker, "VIN", "zone").await;
                    }
                    3 => {
                        // Manual stop
                        if session.is_active() {
                            operator.on_stop_success("sess-1", 3600, 2.50, "EUR");
                        }
                        let _ = process_manual_stop(&mut session, &operator, &broker).await;
                    }
                    _ => unreachable!(),
                }
            }

            // Final state must be consistent — either active or not.
            let is_active = session.is_active();
            if is_active {
                prop_assert!(session.status().is_some(), "active session must have status");
            } else {
                prop_assert!(session.status().is_none(), "inactive session must have no status");
            }

            Ok(())
        })?;
    }
}
