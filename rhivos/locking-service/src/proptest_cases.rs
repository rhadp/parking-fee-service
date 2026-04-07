use proptest::prelude::*;

use crate::command::{parse_command, validate_command, Action, LockCommand};
use crate::process::process_command;
use crate::safety::{check_safety, SafetyResult};
use crate::testing::MockBrokerClient;

fn make_cmd(action: Action, command_id: &str) -> LockCommand {
    LockCommand {
        command_id: command_id.to_string(),
        action,
        doors: vec!["driver".to_string()],
        source: None,
        vin: None,
        timestamp: None,
    }
}

proptest! {
    /// TS-03-P1: Any string input either parses to a valid LockCommand or is rejected.
    #[test]
    #[ignore]
    fn proptest_command_validation_completeness(input in "\\PC*") {
        match parse_command(&input) {
            Ok(cmd) => {
                match validate_command(&cmd) {
                    Ok(()) => {
                        prop_assert!(!cmd.command_id.is_empty());
                        prop_assert!(cmd.action == Action::Lock || cmd.action == Action::Unlock);
                        prop_assert!(cmd.doors.contains(&"driver".to_string()));
                    }
                    Err(_) => {
                        // Rejected by validation — acceptable
                    }
                }
            }
            Err(_) => {
                // Rejected by parsing — acceptable
            }
        }
    }

    /// TS-03-P2: Lock is allowed iff speed < 1.0 and door closed. Speed check takes priority.
    #[test]
    #[ignore]
    fn proptest_safety_gate_lock(speed in 0.0f32..200.0f32, door_open: bool) {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open));
            let result = check_safety(&mock).await;
            if speed >= 1.0 {
                prop_assert_eq!(result, SafetyResult::VehicleMoving);
            } else if door_open {
                prop_assert_eq!(result, SafetyResult::DoorOpen);
            } else {
                prop_assert_eq!(result, SafetyResult::Safe);
            }
            Ok(())
        })?;
    }

    /// TS-03-P3: Unlock succeeds regardless of speed and door state.
    #[test]
    #[ignore]
    fn proptest_unlock_always_succeeds(speed in 0.0f32..200.0f32, door_open: bool) {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open))
                .with_locked(Some(true));
            let mut lock_state = true;
            let cmd = make_cmd(Action::Unlock, "proptest-unlock");
            let response = process_command(&mock, &cmd, &mut lock_state).await;
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
            Ok(())
        })?;
    }

    /// TS-03-P4: After a successful command, lock_state matches the requested action.
    #[test]
    #[ignore]
    fn proptest_state_response_consistency(action_is_lock: bool) {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(0.0))
                .with_door_open(Some(false));
            let mut lock_state = false;
            let action = if action_is_lock { Action::Lock } else { Action::Unlock };
            let cmd = make_cmd(action, "proptest-consistency");
            let response = process_command(&mock, &cmd, &mut lock_state).await;
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
            if action_is_lock {
                prop_assert!(lock_state);
            } else {
                prop_assert!(!lock_state);
            }
            Ok(())
        })?;
    }

    /// TS-03-P5: Repeating the same command N times results in at most one state write.
    #[test]
    #[ignore]
    fn proptest_idempotent_operations(action_is_lock: bool, n in 1usize..5) {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(0.0))
                .with_door_open(Some(false));
            let mut lock_state = false;
            let action = if action_is_lock { Action::Lock } else { Action::Unlock };
            let cmd = make_cmd(action, "proptest-idempotent");
            for _ in 0..n {
                let response = process_command(&mock, &cmd, &mut lock_state).await;
                let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
                prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
            }
            prop_assert!(mock.set_bool_calls().len() <= 1);
            Ok(())
        })?;
    }

    /// TS-03-P6: Every processed command produces a response with required fields.
    #[test]
    #[ignore]
    fn proptest_response_completeness(
        action_is_lock: bool,
        speed in proptest::prop_oneof![Just(0.0f32), Just(50.0f32)],
        door_open: bool,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open));
            let mut lock_state = false;
            let action = if action_is_lock { Action::Lock } else { Action::Unlock };
            let cmd = make_cmd(action, "proptest-complete");
            let response = process_command(&mock, &cmd, &mut lock_state).await;
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert!(parsed["command_id"].as_str().is_some());
            prop_assert_eq!(parsed["command_id"].as_str().unwrap(), "proptest-complete");
            let status = parsed["status"].as_str().unwrap();
            prop_assert!(status == "success" || status == "failed");
            prop_assert!(parsed["timestamp"].as_i64().unwrap() > 0);
            // Verify exactly one set_string call (response publish)
            prop_assert_eq!(mock.set_string_calls().len(), 1);
            Ok(())
        })?;
    }
}
