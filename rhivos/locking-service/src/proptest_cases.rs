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
    // TS-03-P1: Command Validation Completeness
    // Any string input either parses to a valid LockCommand or is rejected.
    #[test]
    #[ignore]
    fn proptest_command_validation_completeness(input in ".*") {
        match parse_command(&input) {
            Ok(cmd) => {
                match validate_command(&cmd) {
                    Ok(()) => {
                        prop_assert!(!cmd.command_id.is_empty());
                        prop_assert!(cmd.action == Action::Lock || cmd.action == Action::Unlock);
                        prop_assert!(cmd.doors.contains(&"driver".to_string()));
                    }
                    Err(_) => {
                        // Rejected by validation — fine.
                    }
                }
            }
            Err(_) => {
                // Rejected by parsing — fine.
            }
        }
    }

    // TS-03-P2: Safety Gate for Lock
    // Lock is allowed iff speed < 1.0 and door closed. Speed check takes priority.
    #[test]
    #[ignore]
    fn proptest_safety_gate_lock(
        speed in 0.0f32..200.0f32,
        door_open in proptest::bool::ANY,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
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

    // TS-03-P3: Unlock Always Succeeds
    // Unlock succeeds regardless of speed and door state.
    #[test]
    #[ignore]
    fn proptest_unlock_always_succeeds(
        speed in 0.0f32..200.0f32,
        door_open in proptest::bool::ANY,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open))
                .with_locked(Some(true));
            let mut lock_state = true;
            let cmd = make_cmd(Action::Unlock, "prop-unlock");
            let response = process_command(&mock, &cmd, &mut lock_state).await;
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
            Ok(())
        })?;
    }

    // TS-03-P4: State-Response Consistency
    // After a successful command, lock_state matches the requested action.
    #[test]
    #[ignore]
    fn proptest_state_response_consistency(action_is_lock in proptest::bool::ANY) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(0.0))
                .with_door_open(Some(false));
            let mut lock_state = false;
            let action = if action_is_lock { Action::Lock } else { Action::Unlock };
            let cmd = make_cmd(action, "prop-consistency");
            let response = process_command(&mock, &cmd, &mut lock_state).await;
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
            if action_is_lock {
                prop_assert!(lock_state, "lock_state should be true after lock");
            } else {
                prop_assert!(!lock_state, "lock_state should be false after unlock");
            }
            Ok(())
        })?;
    }

    // TS-03-P5: Idempotent Operations
    // Repeating the same command N times results in at most one state write.
    #[test]
    #[ignore]
    fn proptest_idempotent_operations(
        action_is_lock in proptest::bool::ANY,
        n in 1usize..5,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(0.0))
                .with_door_open(Some(false));
            let mut lock_state = false;
            let action = if action_is_lock { Action::Lock } else { Action::Unlock };
            for _ in 0..n {
                let cmd = make_cmd(action.clone(), "prop-idempotent");
                let response = process_command(&mock, &cmd, &mut lock_state).await;
                let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
                prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
            }
            prop_assert!(
                mock.set_bool_calls().len() <= 1,
                "set_bool should be called at most once, got {}",
                mock.set_bool_calls().len()
            );
            Ok(())
        })?;
    }

    // TS-03-P6: Response Completeness
    // Every processed command produces exactly one response with required fields.
    #[test]
    #[ignore]
    fn proptest_response_completeness(
        action_is_lock in proptest::bool::ANY,
        speed in prop_oneof![Just(0.0f32), Just(50.0f32)],
        door_open in proptest::bool::ANY,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open));
            let mut lock_state = false;
            let action = if action_is_lock { Action::Lock } else { Action::Unlock };
            let cmd = make_cmd(action, "prop-completeness");
            let response = process_command(&mock, &cmd, &mut lock_state).await;
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert_eq!(parsed["command_id"].as_str().unwrap(), "prop-completeness");
            let status = parsed["status"].as_str().unwrap();
            prop_assert!(
                status == "success" || status == "failed",
                "status must be success or failed, got {status}"
            );
            prop_assert!(parsed["timestamp"].as_i64().unwrap() > 0);
            prop_assert_eq!(
                mock.set_string_calls().len(), 1,
                "exactly one set_string call expected"
            );
            Ok(())
        })?;
    }
}
