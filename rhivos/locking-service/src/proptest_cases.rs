use proptest::prelude::*;

use crate::command::{parse_command, validate_command, Action, LockCommand};
use crate::process::process_command;
use crate::safety::{check_safety, SafetyResult};
use crate::testing::MockBrokerClient;

proptest! {
    // TS-03-P1: Command validation completeness.
    // Any string input either parses to a valid LockCommand or is rejected.
    #[test]
    #[ignore]
    fn proptest_command_validation_completeness(input in "\\PC*") {
        match parse_command(&input) {
            Ok(cmd) => {
                if let Ok(()) = validate_command(&cmd) {
                    prop_assert!(!cmd.command_id.is_empty());
                    prop_assert!(cmd.action == Action::Lock || cmd.action == Action::Unlock);
                    prop_assert!(cmd.doors.contains(&"driver".to_string()));
                }
                // Err from validate_command is fine (rejected)
            }
            Err(_) => {} // Rejected is fine
        }
    }

    // TS-03-P2: Safety gate for lock.
    // Lock is allowed iff speed < 1.0 and door closed. Speed check takes priority.
    #[test]
    #[ignore]
    fn proptest_safety_gate_lock(speed in 0.0f32..200.0f32, door_open in any::<bool>()) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open));
            check_safety(&mock).await
        });
        if speed >= 1.0 {
            prop_assert_eq!(result, SafetyResult::VehicleMoving);
        } else if door_open {
            prop_assert_eq!(result, SafetyResult::DoorOpen);
        } else {
            prop_assert_eq!(result, SafetyResult::Safe);
        }
    }

    // TS-03-P3: Unlock always succeeds regardless of speed and door state.
    #[test]
    #[ignore]
    fn proptest_unlock_always_succeeds(speed in 0.0f32..200.0f32, door_open in any::<bool>()) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mock = MockBrokerClient::new()
            .with_speed(Some(speed))
            .with_door_open(Some(door_open));
        let mut lock_state = true;
        let cmd = LockCommand {
            command_id: "prop-test-unlock".to_string(),
            action: Action::Unlock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        };
        let response = rt.block_on(process_command(&mock, &cmd, &mut lock_state));
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        prop_assert!(parsed["status"] == "success");
    }

    // TS-03-P4: State-response consistency.
    // After a successful command, lock_state matches the requested action.
    #[test]
    #[ignore]
    fn proptest_state_response_consistency(action_is_lock in any::<bool>()) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;
        let cmd = LockCommand {
            command_id: "prop-test-state".to_string(),
            action: if action_is_lock { Action::Lock } else { Action::Unlock },
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        };
        let response = rt.block_on(process_command(&mock, &cmd, &mut lock_state));
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        prop_assert!(parsed["status"] == "success");
        if action_is_lock {
            prop_assert!(lock_state, "lock_state should be true after lock");
        } else {
            prop_assert!(!lock_state, "lock_state should be false after unlock");
        }
    }

    // TS-03-P5: Idempotent operations.
    // Repeating the same command N times results in at most one state write.
    #[test]
    #[ignore]
    fn proptest_idempotent_operations(action_is_lock in any::<bool>(), n in 1usize..5) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;
        let cmd = LockCommand {
            command_id: "prop-test-idempotent".to_string(),
            action: if action_is_lock { Action::Lock } else { Action::Unlock },
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        };
        for _ in 0..n {
            let response = rt.block_on(process_command(&mock, &cmd, &mut lock_state));
            let parsed: serde_json::Value =
                serde_json::from_str(&response).expect("valid JSON");
            prop_assert!(parsed["status"] == "success");
        }
        prop_assert!(
            mock.set_bool_calls().len() <= 1,
            "at most one set_bool call expected, got {}",
            mock.set_bool_calls().len()
        );
    }

    // TS-03-P6: Response completeness.
    // Every processed command produces exactly one response with required fields.
    #[test]
    #[ignore]
    fn proptest_response_completeness(
        action_is_lock in any::<bool>(),
        speed_idx in 0usize..2,
        door_open in any::<bool>(),
    ) {
        let speed = if speed_idx == 0 { 0.0f32 } else { 50.0f32 };
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mock = MockBrokerClient::new()
            .with_speed(Some(speed))
            .with_door_open(Some(door_open));
        let mut lock_state = false;
        let cmd = LockCommand {
            command_id: "prop-test-response".to_string(),
            action: if action_is_lock { Action::Lock } else { Action::Unlock },
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        };
        let response = rt.block_on(process_command(&mock, &cmd, &mut lock_state));
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        prop_assert!(parsed["command_id"] == "prop-test-response");
        prop_assert!(parsed["status"] == "success" || parsed["status"] == "failed");
        prop_assert!(parsed["timestamp"].as_i64().unwrap() > 0);
        prop_assert_eq!(
            mock.set_string_calls().len(),
            1,
            "exactly one set_string call expected for response"
        );
    }
}
