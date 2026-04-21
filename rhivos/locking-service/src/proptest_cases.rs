//! Property-based tests verifying invariants across randomised inputs.
//! All tests are marked `#[ignore]`; run with: cargo test -- --ignored

#[cfg(test)]
mod tests {
    use crate::command::{parse_command, validate_command, Action, LockCommand};
    use crate::process::process_command;
    use crate::safety::check_safety;
    use crate::testing::MockBrokerClient;
    use proptest::prelude::*;

    fn make_lock_cmd(id: &str) -> LockCommand {
        LockCommand {
            command_id: id.to_string(),
            action: Action::Lock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    fn make_unlock_cmd(id: &str) -> LockCommand {
        LockCommand {
            command_id: id.to_string(),
            action: Action::Unlock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    // TS-03-P1: Command Validation Completeness
    // Any string input either parses+validates to a well-formed command, or is rejected.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_command_validation_completeness(input in ".*") {
            match parse_command(&input) {
                Ok(cmd) => {
                    match validate_command(&cmd) {
                        Ok(()) => {
                            assert!(!cmd.command_id.is_empty(), "valid cmd must have non-empty command_id");
                            assert!(
                                cmd.doors.iter().all(|d| d == "driver"),
                                "valid cmd doors must all be 'driver'"
                            );
                        }
                        Err(_) => {} // rejected is acceptable
                    }
                }
                Err(_) => {} // rejected is acceptable
            }
        }
    }

    // TS-03-P2: Safety Gate for Lock
    // Lock allowed iff speed < 1.0 AND door closed. Speed check takes priority.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_safety_gate_lock(speed in 0.0f32..200.0f32, door_open in proptest::bool::ANY) {
            use crate::safety::SafetyResult;
            let result = tokio_test::block_on(async {
                let mock = MockBrokerClient::new()
                    .with_speed(speed)
                    .with_door_open(door_open);
                check_safety(&mock).await
            });
            if speed < 1.0 && !door_open {
                prop_assert_eq!(result, SafetyResult::Safe);
            } else if speed >= 1.0 {
                prop_assert_eq!(result, SafetyResult::VehicleMoving, "speed >= 1.0 must yield VehicleMoving");
            } else {
                prop_assert_eq!(result, SafetyResult::DoorOpen);
            }
        }
    }

    // TS-03-P3: Unlock Always Succeeds
    // Unlock succeeds regardless of speed and door state.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_unlock_always_succeeds(speed in 0.0f32..200.0f32, door_open in proptest::bool::ANY) {
            let response = tokio_test::block_on(async {
                let mock = MockBrokerClient::new()
                    .with_speed(speed)
                    .with_door_open(door_open)
                    .with_locked(true);
                let mut lock_state = true;
                process_command(&mock, &make_unlock_cmd("p3-id"), &mut lock_state).await
            });
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
        }
    }

    // TS-03-P4: State-Response Consistency
    // After a successful command, lock_state matches the requested action.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_state_response_consistency(action_is_lock in proptest::bool::ANY) {
            let (response, lock_state) = tokio_test::block_on(async {
                let mock = MockBrokerClient::new()
                    .with_speed(0.0)
                    .with_door_open(false);
                let mut ls = false;
                let cmd = if action_is_lock {
                    make_lock_cmd("p4-id")
                } else {
                    make_unlock_cmd("p4-id")
                };
                let resp = process_command(&mock, &cmd, &mut ls).await;
                (resp, ls)
            });
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
            if action_is_lock {
                prop_assert!(lock_state, "lock_state must be true after lock");
            } else {
                prop_assert!(!lock_state, "lock_state must be false after unlock");
            }
        }
    }

    // TS-03-P5: Idempotent Operations
    // Repeating the same command N times results in at most one set_bool call.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_idempotent_operations(
            action_is_lock in proptest::bool::ANY,
            n in 1usize..5
        ) {
            let set_bool_count = tokio_test::block_on(async {
                let mock = MockBrokerClient::new()
                    .with_speed(0.0)
                    .with_door_open(false);
                let mut lock_state = false;
                let cmd = if action_is_lock {
                    make_lock_cmd("p5-id")
                } else {
                    make_unlock_cmd("p5-id")
                };
                for _ in 0..n {
                    let resp = process_command(&mock, &cmd, &mut lock_state).await;
                    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
                    assert_eq!(parsed["status"].as_str().unwrap(), "success");
                }
                mock.set_bool_calls().len()
            });
            prop_assert!(
                set_bool_count <= 1,
                "set_bool called {set_bool_count} times, expected at most 1"
            );
        }
    }

    // TS-03-P6: Response Completeness
    // Every processed command produces exactly one response with required fields.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_response_completeness(
            action_is_lock in proptest::bool::ANY,
            speed in proptest::sample::select(vec![0.0f32, 50.0f32]),
            door_open in proptest::bool::ANY
        ) {
            use crate::broker::SIGNAL_RESPONSE;
            let (response, set_string_count) = tokio_test::block_on(async {
                let mock = MockBrokerClient::new()
                    .with_speed(speed)
                    .with_door_open(door_open);
                let mut lock_state = false;
                let cmd = if action_is_lock {
                    make_lock_cmd("p6-id")
                } else {
                    make_unlock_cmd("p6-id")
                };
                let resp = process_command(&mock, &cmd, &mut lock_state).await;
                let count = mock
                    .set_string_calls()
                    .iter()
                    .filter(|(sig, _)| sig == SIGNAL_RESPONSE)
                    .count();
                (resp, count)
            });
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert_eq!(parsed["command_id"].as_str().unwrap(), "p6-id");
            let status = parsed["status"].as_str().unwrap();
            prop_assert!(
                status == "success" || status == "failed",
                "status must be success or failed, got: {status}"
            );
            prop_assert!(
                parsed["timestamp"].is_number(),
                "timestamp must be present"
            );
            prop_assert_eq!(set_string_count, 1, "exactly one response must be published");
        }
    }
}
