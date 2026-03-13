/// Property-based tests for the locking service.
/// These use the proptest crate and are marked #[ignore] for separate runs.
#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::command::{parse_command, validate_command, Action, LockCommand};
    use crate::process::process_command;
    use crate::safety::check_safety;
    use crate::testing::MockBrokerClient;

    // TS-03-P1: Command Validation Completeness
    // Any string input either parses to a valid command or is rejected.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_command_validation_completeness(input in ".*") {
            let result = parse_command(&input);
            match result {
                Ok(cmd) => {
                    // If parsing succeeded, validate it
                    match validate_command(&cmd) {
                        Ok(()) => {
                            prop_assert!(!cmd.command_id.is_empty());
                            prop_assert!(cmd.action == Action::Lock || cmd.action == Action::Unlock);
                            prop_assert!(cmd.doors.contains(&"driver".to_string()));
                        }
                        Err(_) => {
                            // Validation failed — that's fine, it was rejected
                        }
                    }
                }
                Err(_) => {
                    // Parse failed — that's fine, it was rejected
                }
            }
        }
    }

    // TS-03-P2: Safety Gate for Lock
    // Lock is allowed iff speed < 1.0 and door closed.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_safety_gate_lock(
            speed in 0.0f32..200.0f32,
            door_open in proptest::bool::ANY,
        ) {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open));
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(check_safety(&mock));

            use crate::safety::SafetyResult;
            if speed < 1.0 && !door_open {
                prop_assert_eq!(result, SafetyResult::Safe);
            } else if speed >= 1.0 {
                prop_assert_eq!(result, SafetyResult::VehicleMoving);
            } else {
                prop_assert_eq!(result, SafetyResult::DoorOpen);
            }
        }
    }

    // TS-03-P3: Unlock Always Succeeds
    proptest! {
        #[test]
        #[ignore]
        fn proptest_unlock_always_succeeds(
            speed in 0.0f32..200.0f32,
            door_open in proptest::bool::ANY,
        ) {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open))
                .with_locked(true);

            let unlock_cmd = LockCommand {
                command_id: "prop-unlock".to_string(),
                action: Action::Unlock,
                doors: vec!["driver".to_string()],
                source: None,
                vin: None,
                timestamp: None,
            };

            let rt = tokio::runtime::Runtime::new().unwrap();
            let response = rt.block_on(async {
                let mut lock_state = true;
                process_command(&mock, &unlock_cmd, &mut lock_state).await
            });

            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert!(parsed["status"] == "success");
        }
    }

    // TS-03-P4: State-Response Consistency
    proptest! {
        #[test]
        #[ignore]
        fn proptest_state_response_consistency(
            action_is_lock in proptest::bool::ANY,
        ) {
            let mock = MockBrokerClient::new()
                .with_speed(Some(0.0))
                .with_door_open(Some(false));

            let cmd = LockCommand {
                command_id: "prop-consistency".to_string(),
                action: if action_is_lock { Action::Lock } else { Action::Unlock },
                doors: vec!["driver".to_string()],
                source: None,
                vin: None,
                timestamp: None,
            };

            let rt = tokio::runtime::Runtime::new().unwrap();
            let (response, final_state) = rt.block_on(async {
                let mut lock_state = false;
                let resp = process_command(&mock, &cmd, &mut lock_state).await;
                (resp, lock_state)
            });

            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert!(parsed["status"] == "success");

            if action_is_lock {
                prop_assert!(final_state, "lock action should set state to true");
            } else {
                prop_assert!(!final_state, "unlock action should set state to false");
            }
        }
    }

    // TS-03-P5: Idempotent Operations
    proptest! {
        #[test]
        #[ignore]
        fn proptest_idempotent_operations(
            action_is_lock in proptest::bool::ANY,
            n in 1usize..5usize,
        ) {
            let mock = MockBrokerClient::new()
                .with_speed(Some(0.0))
                .with_door_open(Some(false));

            let cmd = LockCommand {
                command_id: "prop-idempotent".to_string(),
                action: if action_is_lock { Action::Lock } else { Action::Unlock },
                doors: vec!["driver".to_string()],
                source: None,
                vin: None,
                timestamp: None,
            };

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut lock_state = false;
                for _ in 0..n {
                    let response = process_command(&mock, &cmd, &mut lock_state).await;
                    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
                    assert_eq!(parsed["status"], "success");
                }
                // State should have been set at most once
                let calls = mock.set_bool_calls();
                assert!(calls.len() <= 1, "set_bool should be called at most once, got {}", calls.len());
            });
        }
    }

    // TS-03-P6: Response Completeness
    proptest! {
        #[test]
        #[ignore]
        fn proptest_response_completeness(
            action_is_lock in proptest::bool::ANY,
            speed in proptest::prop_oneof![Just(0.0f32), Just(50.0f32)],
            door_open in proptest::bool::ANY,
        ) {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open));

            let cmd = LockCommand {
                command_id: "test-id".to_string(),
                action: if action_is_lock { Action::Lock } else { Action::Unlock },
                doors: vec!["driver".to_string()],
                source: None,
                vin: None,
                timestamp: None,
            };

            let rt = tokio::runtime::Runtime::new().unwrap();
            let response = rt.block_on(async {
                let mut lock_state = false;
                process_command(&mock, &cmd, &mut lock_state).await
            });

            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert!(parsed["command_id"] == "test-id");
            prop_assert!(
                parsed["status"] == "success" || parsed["status"] == "failed",
                "status must be success or failed"
            );
            prop_assert!(parsed["timestamp"].as_i64().unwrap() > 0);

            // Exactly one response was published
            let string_calls = mock.set_string_calls();
            prop_assert_eq!(string_calls.len(), 1, "exactly one response should be published");
        }
    }
}
