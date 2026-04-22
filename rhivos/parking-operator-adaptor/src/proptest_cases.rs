use proptest::prelude::*;

use crate::event_loop::{process_lock_event, process_manual_start, process_manual_stop};
use crate::operator::OperatorError;
use crate::session::{Rate, Session};
use crate::testing::{
    make_start_response, make_stop_response, MockBrokerClient, MockOperatorClient,
};

proptest! {
    // TS-08-P1: Session state consistency.
    // After any sequence of start/stop operations, session.is_active()
    // matches the last successful operation.
    #[test]
    #[ignore]
    fn proptest_session_state_consistency(
        ops in prop::collection::vec(any::<bool>(), 1..20),
    ) {
        let mut session = Session::new();
        let mut last_successful: Option<bool> = None; // true = start, false = stop

        for is_start in &ops {
            if *is_start {
                let rate = Rate {
                    rate_type: "per_hour".to_string(),
                    amount: 2.5,
                    currency: "EUR".to_string(),
                };
                session.start("s1".to_string(), "z1".to_string(), 1_700_000_000, rate);
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

    // TS-08-P2: Idempotent lock events.
    // N consecutive lock events followed by M consecutive unlock events:
    // operator start called exactly once, stop called exactly once.
    #[test]
    #[ignore]
    fn proptest_idempotent_lock_events(n in 1usize..10, m in 1usize..10) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mock_broker = MockBrokerClient::new();
            let mock_operator = MockOperatorClient::new();

            // Pre-load enough start and stop responses.
            for _ in 0..n {
                mock_operator.on_start_return(Ok(make_start_response("sess-1")));
            }
            for _ in 0..m {
                mock_operator.on_stop_return(Ok(make_stop_response("sess-1")));
            }

            let mut session = Session::new();

            // N lock events.
            for _ in 0..n {
                let _ = process_lock_event(
                    true,
                    &mut session,
                    &mock_operator,
                    &mock_broker,
                    "VIN",
                    "zone",
                )
                .await;
            }
            prop_assert_eq!(
                mock_operator.start_call_count(),
                1,
                "start should be called exactly once for {} lock events",
                n
            );

            // M unlock events.
            for _ in 0..m {
                let _ = process_lock_event(
                    false,
                    &mut session,
                    &mock_operator,
                    &mock_broker,
                    "VIN",
                    "zone",
                )
                .await;
            }
            prop_assert_eq!(
                mock_operator.stop_call_count(),
                1,
                "stop should be called exactly once for {} unlock events",
                m
            );

            Ok(())
        })?;
    }

    // TS-08-P3: Override non-persistence.
    // After any manual override, the next lock/unlock cycle resumes autonomous behavior.
    #[test]
    #[ignore]
    fn proptest_override_non_persistence(use_manual_start in any::<bool>()) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mock_broker = MockBrokerClient::new();
            let mock_operator = MockOperatorClient::new();
            let mut session = Session::new();

            if use_manual_start {
                // Manual start then manual stop.
                mock_operator.on_start_return(Ok(make_start_response("sess-1")));
                let _ = process_manual_start(
                    "zone",
                    &mut session,
                    &mock_operator,
                    &mock_broker,
                    "VIN",
                )
                .await;
                mock_operator.on_stop_return(Ok(make_stop_response("sess-1")));
                let _ = process_manual_stop(&mut session, &mock_operator, &mock_broker).await;
            } else {
                // Start a session, then manual stop.
                let rate = Rate {
                    rate_type: "per_hour".to_string(),
                    amount: 2.5,
                    currency: "EUR".to_string(),
                };
                session.start("sess-1".to_string(), "z1".to_string(), 1_700_000_000, rate);
                mock_operator.on_stop_return(Ok(make_stop_response("sess-1")));
                let _ = process_manual_stop(&mut session, &mock_operator, &mock_broker).await;
            }

            // Next lock should start autonomous session.
            mock_operator.on_start_return(Ok(make_start_response("sess-2")));
            let result = process_lock_event(
                true,
                &mut session,
                &mock_operator,
                &mock_broker,
                "VIN",
                "zone",
            )
            .await;
            prop_assert!(result.is_ok(), "autonomous start should succeed");
            prop_assert!(session.is_active(), "session should be active after autonomous lock");

            // Next unlock should stop autonomous session.
            mock_operator.on_stop_return(Ok(make_stop_response("sess-2")));
            let result = process_lock_event(
                false,
                &mut session,
                &mock_operator,
                &mock_broker,
                "VIN",
                "zone",
            )
            .await;
            prop_assert!(result.is_ok(), "autonomous stop should succeed");
            prop_assert!(!session.is_active(), "session should be inactive after autonomous unlock");

            Ok(())
        })?;
    }

    // TS-08-P4: Retry exhaustion safety.
    // Failed REST calls never corrupt session state.
    #[test]
    #[ignore]
    fn proptest_retry_exhaustion_safety(failure_type in 0u8..3) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mock_broker = MockBrokerClient::new();
            let mock_operator = MockOperatorClient::new();
            let mut session = Session::new();

            let error = match failure_type {
                0 => OperatorError::RequestFailed("timeout".to_string()),
                1 => OperatorError::HttpError(500, "server error".to_string()),
                _ => OperatorError::RequestFailed("connection refused".to_string()),
            };
            mock_operator.on_start_return(Err(error));

            let was_active_before = session.is_active();

            let _result = process_lock_event(
                true,
                &mut session,
                &mock_operator,
                &mock_broker,
                "VIN",
                "zone",
            )
            .await;

            // State should be unchanged after failure.
            prop_assert_eq!(
                session.is_active(),
                was_active_before,
                "session state should be unchanged after operator failure"
            );

            Ok(())
        })?;
    }

    // TS-08-P5: SessionActive signal consistency.
    // After any sequence of successful start/stop, the last set_bool call
    // matches session.is_active().
    #[test]
    #[ignore]
    fn proptest_session_active_consistency(
        ops in prop::collection::vec(any::<bool>(), 1..10),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mock_broker = MockBrokerClient::new();
            let mock_operator = MockOperatorClient::new();
            let mut session = Session::new();
            let mut session_counter = 0u32;

            for &is_lock in &ops {
                if is_lock && !session.is_active() {
                    session_counter += 1;
                    let id = format!("sess-{session_counter}");
                    mock_operator.on_start_return(Ok(make_start_response(&id)));
                    let _ = process_lock_event(
                        true,
                        &mut session,
                        &mock_operator,
                        &mock_broker,
                        "VIN",
                        "zone",
                    )
                    .await;
                } else if !is_lock && session.is_active() {
                    let id = session.status().unwrap().session_id.clone();
                    mock_operator.on_stop_return(Ok(make_stop_response(&id)));
                    let _ = process_lock_event(
                        false,
                        &mut session,
                        &mock_operator,
                        &mock_broker,
                        "VIN",
                        "zone",
                    )
                    .await;
                }
            }

            // Check that the last SessionActive value matches session state.
            if let Some(last_value) = mock_broker.last_session_active_value() {
                prop_assert_eq!(
                    last_value,
                    session.is_active(),
                    "last SessionActive should match session.is_active()"
                );
            }

            Ok(())
        })?;
    }

    // TS-08-P6: Sequential event processing.
    // Processing events sequentially produces deterministic results.
    // (This property test validates outcome determinism; the mechanism guarantee
    // of at-most-one-in-flight is an architecture-level invariant verified
    // by the event loop channel design.)
    #[test]
    #[ignore]
    fn proptest_sequential_event_processing(
        events in prop::collection::vec(any::<bool>(), 2..5),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        // Run twice with same events — result must be identical.
        let run = |events: &[bool]| {
            rt.block_on(async {
                let mock_broker = MockBrokerClient::new();
                let mock_operator = MockOperatorClient::new();
                let mut session = Session::new();
                let mut counter = 0u32;

                for &is_lock in events {
                    if is_lock && !session.is_active() {
                        counter += 1;
                        mock_operator
                            .on_start_return(Ok(make_start_response(&format!("s-{counter}"))));
                    } else if !is_lock && session.is_active() {
                        let id = session.status().unwrap().session_id.clone();
                        mock_operator.on_stop_return(Ok(make_stop_response(&id)));
                    }
                    let _ = process_lock_event(
                        is_lock,
                        &mut session,
                        &mock_operator,
                        &mock_broker,
                        "VIN",
                        "zone",
                    )
                    .await;
                }

                session.is_active()
            })
        };

        let result1 = run(&events);
        let result2 = run(&events);
        prop_assert_eq!(result1, result2, "sequential processing should be deterministic");
    }
}
