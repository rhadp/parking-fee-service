#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::broker::SIGNAL_RESPONSE;
    use crate::command::{parse_command, validate_command, Action, LockCommand};
    use crate::process::process_command;
    use crate::safety::{check_safety, SafetyResult};
    use crate::testing::MockBrokerClient;

    // TS-03-P1: Command Validation Completeness
    // For any string input, parse_command either rejects it or returns a LockCommand
    // that validate_command either accepts (with valid fields) or rejects.
    #[test]
    #[ignore]
    fn proptest_command_validation_completeness() {
        proptest!(|(input: String)| {
            if let Ok(cmd) = parse_command(&input) {
                if let Ok(()) = validate_command(&cmd) {
                    prop_assert!(!cmd.command_id.is_empty());
                    prop_assert!(
                        cmd.action == Action::Lock || cmd.action == Action::Unlock
                    );
                    prop_assert!(cmd.doors.contains(&"driver".to_string()));
                }
            }
        });
    }

    // TS-03-P2: Safety Gate for Lock
    // Lock is allowed iff speed < 1.0 and door closed. Speed check takes priority.
    #[test]
    #[ignore]
    fn proptest_safety_gate_lock() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        proptest!(|(speed in 0.0_f32..200.0_f32, door_open: bool)| {
            let mock = MockBrokerClient::new()
                .with_speed(speed)
                .with_door_open(door_open);
            let result = rt.block_on(check_safety(&mock));
            if speed >= 1.0 {
                prop_assert_eq!(result, SafetyResult::VehicleMoving);
            } else if door_open {
                prop_assert_eq!(result, SafetyResult::DoorOpen);
            } else {
                prop_assert_eq!(result, SafetyResult::Safe);
            }
        });
    }

    // TS-03-P3: Unlock Always Succeeds
    // Unlock succeeds regardless of speed and door state.
    #[test]
    #[ignore]
    fn proptest_unlock_always_succeeds() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        proptest!(|(speed in 0.0_f32..200.0_f32, door_open: bool)| {
            let mock = MockBrokerClient::new()
                .with_speed(speed)
                .with_door_open(door_open)
                .with_locked(true);
            let mut lock_state = true;
            let cmd = LockCommand {
                command_id: "prop-test".to_string(),
                action: Action::Unlock,
                doors: vec!["driver".to_string()],
                source: None,
                vin: None,
                timestamp: None,
            };
            let response = rt.block_on(process_command(&mock, &cmd, &mut lock_state));
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert!(parsed["status"] == "success");
        });
    }

    // TS-03-P4: State-Response Consistency
    // After a successful command, lock_state matches the requested action.
    #[test]
    #[ignore]
    fn proptest_state_response_consistency() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        proptest!(|(action_is_lock: bool)| {
            let mock = MockBrokerClient::new()
                .with_speed(0.0_f32)
                .with_door_open(false);
            let mut lock_state = false;
            let action = if action_is_lock {
                Action::Lock
            } else {
                Action::Unlock
            };
            let cmd = LockCommand {
                command_id: "prop-test".to_string(),
                action,
                doors: vec!["driver".to_string()],
                source: None,
                vin: None,
                timestamp: None,
            };
            let response = rt.block_on(process_command(&mock, &cmd, &mut lock_state));
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert!(parsed["status"] == "success");
            if action_is_lock {
                prop_assert!(lock_state, "lock_state should be true after lock");
            } else {
                prop_assert!(!lock_state, "lock_state should be false after unlock");
            }
        });
    }

    // TS-03-P5: Idempotent Operations
    // Repeating the same command N times results in at most one state write.
    // Addresses skeptic review: start locked=true for unlock path so we test
    // the real transition before idempotent repeats.
    #[test]
    #[ignore]
    fn proptest_idempotent_operations() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        proptest!(|(action_is_lock: bool, n in 1_u32..5)| {
            let mock = MockBrokerClient::new()
                .with_speed(0.0_f32)
                .with_door_open(false);
            // Start in opposite state so first command causes a real transition
            let mut lock_state = !action_is_lock;
            let action = if action_is_lock {
                Action::Lock
            } else {
                Action::Unlock
            };
            for _ in 0..n {
                let cmd = LockCommand {
                    command_id: "prop-test".to_string(),
                    action: action.clone(),
                    doors: vec!["driver".to_string()],
                    source: None,
                    vin: None,
                    timestamp: None,
                };
                let response =
                    rt.block_on(process_command(&mock, &cmd, &mut lock_state));
                let parsed: serde_json::Value =
                    serde_json::from_str(&response).unwrap();
                prop_assert!(parsed["status"] == "success");
            }
            prop_assert!(
                mock.set_bool_calls().len() <= 1,
                "set_bool should be called at most once, got {}",
                mock.set_bool_calls().len()
            );
        });
    }

    // TS-03-P6: Response Completeness
    // Every processed command produces exactly one response with required fields.
    #[test]
    #[ignore]
    fn proptest_response_completeness() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        proptest!(|(
            action_is_lock: bool,
            speed_is_high: bool,
            door_open: bool,
        )| {
            let speed = if speed_is_high { 50.0_f32 } else { 0.0_f32 };
            let mock = MockBrokerClient::new()
                .with_speed(speed)
                .with_door_open(door_open);
            let mut lock_state = false;
            let action = if action_is_lock {
                Action::Lock
            } else {
                Action::Unlock
            };
            let cmd = LockCommand {
                command_id: "prop-test".to_string(),
                action,
                doors: vec!["driver".to_string()],
                source: None,
                vin: None,
                timestamp: None,
            };
            let response = rt.block_on(process_command(&mock, &cmd, &mut lock_state));
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert!(parsed["command_id"] == "prop-test");
            let status = parsed["status"].as_str().unwrap();
            prop_assert!(
                status == "success" || status == "failed",
                "status must be 'success' or 'failed', got '{status}'"
            );
            prop_assert!(parsed["timestamp"].as_i64().unwrap() > 0);
            // Verify exactly one response was published
            let response_calls: Vec<_> = mock
                .set_string_calls()
                .iter()
                .filter(|(sig, _)| sig == SIGNAL_RESPONSE)
                .cloned()
                .collect();
            prop_assert_eq!(
                response_calls.len(),
                1,
                "expected exactly one response publish, got {}",
                response_calls.len()
            );
        });
    }
}
