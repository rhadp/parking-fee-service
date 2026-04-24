#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::event_loop::{handle_lock_event, handle_start_session, handle_stop_session};
    use crate::session::{Rate, Session};
    use crate::testing::{
        make_start_response, make_stop_response, MockBrokerClient, MockOperatorClient,
    };

    // TS-08-P1: Session State Consistency
    // Validates: Property 1 (08-REQ-6.1, 08-REQ-6.2, 08-REQ-6.3)
    // After any sequence of start/stop operations, session state matches
    // the last successful operation.
    #[test]
    #[ignore]
    fn proptest_session_state_consistency() {
        proptest!(|(ops in prop::collection::vec(any::<bool>(), 1..20))| {
            let mut session = Session::new();
            let rate = Rate {
                rate_type: "per_hour".to_string(),
                amount: 2.5,
                currency: "EUR".to_string(),
            };

            let mut last_successful: Option<bool> = None; // true=start, false=stop

            for &is_start in &ops {
                if is_start {
                    if !session.is_active() {
                        session.start(
                            "s1".to_string(),
                            "zone-a".to_string(),
                            1_700_000_000,
                            rate.clone(),
                        );
                        last_successful = Some(true);
                    }
                } else if session.is_active() {
                    session.stop();
                    last_successful = Some(false);
                }
            }

            match last_successful {
                Some(true) => prop_assert!(session.is_active()),
                Some(false) | None => prop_assert!(!session.is_active()),
            }
        });
    }

    // TS-08-P2: Idempotent Lock Events
    // Validates: Property 2 (08-REQ-3.E1, 08-REQ-3.E2)
    // Duplicate lock/unlock events are no-ops.
    #[test]
    #[ignore]
    fn proptest_idempotent_lock_events() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(n in 1_usize..10, m in 1_usize..10)| {
            rt.block_on(async {
                let mut session = Session::new();
                let operator = MockOperatorClient::new();
                operator.on_start_return(make_start_response("sess-1"));
                operator.on_stop_return(make_stop_response("sess-1"));
                let broker = MockBrokerClient::new();

                // Send n lock events
                for _ in 0..n {
                    handle_lock_event(
                        true,
                        &mut session,
                        &operator,
                        &broker,
                        "DEMO-VIN-001",
                        "zone-demo-1",
                    )
                    .await;
                }

                // Only one start call should have been made
                prop_assert_eq!(
                    operator.start_call_count(),
                    1,
                    "expected exactly 1 start call after {} lock events",
                    n
                );

                // Send m unlock events
                for _ in 0..m {
                    handle_lock_event(
                        false,
                        &mut session,
                        &operator,
                        &broker,
                        "DEMO-VIN-001",
                        "zone-demo-1",
                    )
                    .await;
                }

                // Only one stop call should have been made
                prop_assert_eq!(
                    operator.stop_call_count(),
                    1,
                    "expected exactly 1 stop call after {} unlock events",
                    m
                );

                Ok(())
            })?;
        });
    }

    // TS-08-P3: Override Non-Persistence
    // Validates: Property 3 (08-REQ-5.1, 08-REQ-5.2, 08-REQ-5.3, 08-REQ-5.E1)
    // After any manual override, the next lock/unlock cycle resumes autonomous behavior.
    #[test]
    #[ignore]
    fn proptest_override_non_persistence() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(override_is_start: bool)| {
            rt.block_on(async {
                let mut session = Session::new();
                let operator = MockOperatorClient::new();
                operator.on_start_return(make_start_response("sess-1"));
                operator.on_stop_return(make_stop_response("sess-1"));
                let broker = MockBrokerClient::new();

                // Apply the manual override
                if override_is_start {
                    let _ = handle_start_session(
                        "zone",
                        &mut session,
                        &operator,
                        &broker,
                        "DEMO-VIN-001",
                    )
                    .await;
                    // Then manual stop
                    let _ = handle_stop_session(&mut session, &operator, &broker).await;
                } else {
                    // Start first, then manual stop
                    let _ = handle_start_session(
                        "zone",
                        &mut session,
                        &operator,
                        &broker,
                        "DEMO-VIN-001",
                    )
                    .await;
                    let _ = handle_stop_session(&mut session, &operator, &broker).await;
                }

                // Reconfigure mock for next cycle
                operator.on_start_return(make_start_response("sess-2"));
                operator.on_stop_return(make_stop_response("sess-2"));

                // Next lock event should resume autonomous behavior
                handle_lock_event(
                    true,
                    &mut session,
                    &operator,
                    &broker,
                    "DEMO-VIN-001",
                    "zone-demo-1",
                )
                .await;
                prop_assert!(session.is_active(), "autonomous start should work after override");

                // Next unlock event should also work
                handle_lock_event(
                    false,
                    &mut session,
                    &operator,
                    &broker,
                    "DEMO-VIN-001",
                    "zone-demo-1",
                )
                .await;
                prop_assert!(!session.is_active(), "autonomous stop should work after override");

                Ok(())
            })?;
        });
    }

    // TS-08-P4: Retry Exhaustion Safety
    // Validates: Property 4 (08-REQ-2.E1, 08-REQ-2.E2)
    // Failed REST calls never corrupt session state.
    #[test]
    #[ignore]
    fn proptest_retry_exhaustion_safety() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(_dummy in 0..3_u32)| {
            rt.block_on(async {
                let mut session = Session::new();
                let operator = MockOperatorClient::new();
                operator.always_fail(); // all calls fail
                let broker = MockBrokerClient::new();

                let was_active_before = session.is_active();

                // Try to start via lock event
                handle_lock_event(
                    true,
                    &mut session,
                    &operator,
                    &broker,
                    "DEMO-VIN-001",
                    "zone-demo-1",
                )
                .await;

                // Session state should be unchanged
                prop_assert_eq!(
                    session.is_active(),
                    was_active_before,
                    "session state must not change after operator failure"
                );

                Ok(())
            })?;
        });
    }

    // TS-08-P5: SessionActive Signal Consistency
    // Validates: Property 5 (08-REQ-4.1, 08-REQ-4.2, 08-REQ-4.3)
    // SessionActive signal always matches session.is_active() after
    // successful operations.
    #[test]
    #[ignore]
    fn proptest_session_active_consistency() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(ops in prop::collection::vec(any::<bool>(), 1..10))| {
            rt.block_on(async {
                let mut session = Session::new();
                let operator = MockOperatorClient::new();
                operator.on_start_return(make_start_response("sess-1"));
                operator.on_stop_return(make_stop_response("sess-1"));
                let broker = MockBrokerClient::new();

                for &is_lock in &ops {
                    handle_lock_event(
                        is_lock,
                        &mut session,
                        &operator,
                        &broker,
                        "DEMO-VIN-001",
                        "zone-demo-1",
                    )
                    .await;
                }

                // The last set_bool call on SessionActive should match
                // session.is_active()
                let calls = broker.set_bool_calls();
                let session_active_calls: Vec<_> = calls
                    .iter()
                    .filter(|(s, _)| s == "Vehicle.Parking.SessionActive")
                    .collect();

                if !session_active_calls.is_empty() {
                    let last_value = session_active_calls.last().unwrap().1;
                    prop_assert_eq!(
                        last_value,
                        session.is_active(),
                        "last SessionActive signal must match session.is_active()"
                    );
                }

                Ok(())
            })?;
        });
    }

    // TS-08-P6: Sequential Event Processing
    // Validates: Property 6 (08-REQ-9.1, 08-REQ-9.2)
    // No concurrent session state mutations occur. Processing events
    // sequentially produces deterministic results.
    #[test]
    #[ignore]
    fn proptest_sequential_event_processing() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(events in prop::collection::vec(any::<bool>(), 2..5))| {
            // Run the same event sequence twice and verify deterministic outcome
            let final_state_1 = rt.block_on(async {
                let mut session = Session::new();
                let operator = MockOperatorClient::new();
                operator.on_start_return(make_start_response("sess-1"));
                operator.on_stop_return(make_stop_response("sess-1"));
                let broker = MockBrokerClient::new();

                for &is_lock in &events {
                    handle_lock_event(
                        is_lock,
                        &mut session,
                        &operator,
                        &broker,
                        "DEMO-VIN-001",
                        "zone-demo-1",
                    )
                    .await;
                }
                session.is_active()
            });

            let final_state_2 = rt.block_on(async {
                let mut session = Session::new();
                let operator = MockOperatorClient::new();
                operator.on_start_return(make_start_response("sess-1"));
                operator.on_stop_return(make_stop_response("sess-1"));
                let broker = MockBrokerClient::new();

                for &is_lock in &events {
                    handle_lock_event(
                        is_lock,
                        &mut session,
                        &operator,
                        &broker,
                        "DEMO-VIN-001",
                        "zone-demo-1",
                    )
                    .await;
                }
                session.is_active()
            });

            prop_assert_eq!(
                final_state_1,
                final_state_2,
                "sequential processing must produce deterministic results"
            );
        });
    }
}
