//! Property-based tests for parking-operator-adaptor.

use proptest::prelude::*;

use crate::event_loop::{process_lock_event, process_manual_start, process_manual_stop};
use crate::session::{Rate, Session};
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

// TS-08-P1: Session State Consistency
// After any sequence of start/stop operations, session state matches the last
// successful operation.
proptest! {
    #[test]
    #[ignore]
    fn proptest_session_state_consistency(
        ops in prop::collection::vec(prop::bool::ANY, 1..10),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
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
                Some(true) => prop_assert!(session.is_active()),
                Some(false) | None => prop_assert!(!session.is_active()),
            }
            Ok(())
        })?;
    }
}

// TS-08-P2: Idempotent Lock Events
// N consecutive lock events produce exactly 1 operator start call.
// M consecutive unlock events produce exactly 1 operator stop call.
proptest! {
    #[test]
    #[ignore]
    fn proptest_idempotent_lock_events(
        n in 1usize..10,
        m in 1usize..10,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let operator = MockOperatorClient::new();
            operator.on_start_return(make_start_response("sess-1"));
            operator.on_stop_return(make_stop_response("sess-1"));
            let broker = MockBrokerClient::new();
            let mut session = Session::new();

            // N lock events
            for _ in 0..n {
                let _ = process_lock_event(
                    true, &mut session, &operator, &broker, "VIN", "zone",
                ).await;
            }
            prop_assert_eq!(operator.start_call_count(), 1, "exactly 1 start call");

            // M unlock events
            for _ in 0..m {
                let _ = process_lock_event(
                    false, &mut session, &operator, &broker, "VIN", "zone",
                ).await;
            }
            prop_assert_eq!(operator.stop_call_count(), 1, "exactly 1 stop call");

            Ok(())
        })?;
    }
}

// TS-08-P3: Override Non-Persistence
// After any manual override, the next lock/unlock cycle resumes autonomous behavior.
proptest! {
    #[test]
    #[ignore]
    fn proptest_override_non_persistence(
        manual_stop_first: bool,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let operator = MockOperatorClient::new();
            operator.on_start_return(make_start_response("sess-1"));
            operator.on_stop_return(make_stop_response("sess-1"));
            let broker = MockBrokerClient::new();
            let mut session = Session::new();

            if manual_stop_first {
                // Start a session first, then manually stop it
                session.start("sess-1".to_string(), "zone".to_string(), 1_700_000_000, sample_rate());
                let _ = process_manual_stop(&mut session, &operator, &broker).await;
                prop_assert!(!session.is_active());
            } else {
                // Manually start
                let _ = process_manual_start("zone", &mut session, &operator, &broker, "VIN").await;
                prop_assert!(session.is_active());
                // Then manually stop
                let _ = process_manual_stop(&mut session, &operator, &broker).await;
            }

            // Reset and configure new response
            operator.reset();
            operator.on_start_return(make_start_response("sess-2"));
            operator.on_stop_return(make_stop_response("sess-2"));

            // Autonomous lock should work
            let _ = process_lock_event(true, &mut session, &operator, &broker, "VIN", "zone").await;
            prop_assert!(session.is_active(), "autonomous start should work after override");
            prop_assert!(operator.start_call_count() >= 1);

            // Autonomous unlock should work
            let _ = process_lock_event(false, &mut session, &operator, &broker, "VIN", "zone").await;
            prop_assert!(!session.is_active(), "autonomous stop should work after override");

            Ok(())
        })?;
    }
}

// TS-08-P4: Retry Exhaustion Safety
// Failed REST calls never corrupt session state.
proptest! {
    #[test]
    #[ignore]
    fn proptest_retry_exhaustion_safety(
        _dummy in 0u8..5,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let operator = MockOperatorClient::new();
            operator.always_fail();
            let broker = MockBrokerClient::new();
            let mut session = Session::new();

            // Try to start session via lock event — should fail
            let result = process_lock_event(
                true, &mut session, &operator, &broker, "VIN", "zone",
            ).await;
            // Whether it returns Ok (noop-like) or Err, state must be unchanged
            let _ = result;
            prop_assert!(!session.is_active(), "session state unchanged after failure");

            Ok(())
        })?;
    }
}

// TS-08-P5: SessionActive Signal Consistency
// SessionActive signal always matches session.active after successful operations.
proptest! {
    #[test]
    #[ignore]
    fn proptest_session_active_consistency(
        ops in prop::collection::vec(prop::bool::ANY, 1..6),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let operator = MockOperatorClient::new();
            operator.on_start_return(make_start_response("sess-1"));
            operator.on_stop_return(make_stop_response("sess-1"));
            let broker = MockBrokerClient::new();
            let mut session = Session::new();

            for is_lock in &ops {
                let _ = process_lock_event(
                    *is_lock, &mut session, &operator, &broker, "VIN", "zone",
                ).await;
            }

            let calls = broker.set_bool_calls();
            if let Some(last) = calls.last() {
                if last.0 == "Vehicle.Parking.SessionActive" {
                    prop_assert_eq!(
                        last.1,
                        session.is_active(),
                        "last SessionActive signal should match session state"
                    );
                }
            }

            Ok(())
        })?;
    }
}

// TS-08-P6: Sequential Event Processing
// Concurrent events produce deterministic results when serialized.
proptest! {
    #[test]
    #[ignore]
    fn proptest_sequential_event_processing(
        events in prop::collection::vec(0u8..4, 2..5),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let operator = MockOperatorClient::new();
            operator.on_start_return(make_start_response("sess-1"));
            operator.on_stop_return(make_stop_response("sess-1"));
            let broker = MockBrokerClient::new();
            let mut session = Session::new();

            // Process events sequentially — result should be deterministic
            for event in &events {
                match event {
                    0 => {
                        // Lock event
                        let _ = process_lock_event(
                            true, &mut session, &operator, &broker, "VIN", "zone",
                        ).await;
                    }
                    1 => {
                        // Unlock event
                        let _ = process_lock_event(
                            false, &mut session, &operator, &broker, "VIN", "zone",
                        ).await;
                    }
                    2 => {
                        // Manual start
                        let _ = process_manual_start(
                            "zone", &mut session, &operator, &broker, "VIN",
                        ).await;
                    }
                    _ => {
                        // Manual stop
                        let _ = process_manual_stop(
                            &mut session, &operator, &broker,
                        ).await;
                    }
                }
            }

            // Replay the same sequence with a fresh state — result should match
            let operator2 = MockOperatorClient::new();
            operator2.on_start_return(make_start_response("sess-1"));
            operator2.on_stop_return(make_stop_response("sess-1"));
            let broker2 = MockBrokerClient::new();
            let mut session2 = Session::new();

            for event in &events {
                match event {
                    0 => {
                        let _ = process_lock_event(
                            true, &mut session2, &operator2, &broker2, "VIN", "zone",
                        ).await;
                    }
                    1 => {
                        let _ = process_lock_event(
                            false, &mut session2, &operator2, &broker2, "VIN", "zone",
                        ).await;
                    }
                    2 => {
                        let _ = process_manual_start(
                            "zone", &mut session2, &operator2, &broker2, "VIN",
                        ).await;
                    }
                    _ => {
                        let _ = process_manual_stop(
                            &mut session2, &operator2, &broker2,
                        ).await;
                    }
                }
            }

            prop_assert_eq!(session.is_active(), session2.is_active(),
                "same event sequence should produce same result");

            Ok(())
        })?;
    }
}
