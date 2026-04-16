//! Property-based tests for LOCKING_SERVICE correctness properties.
//!
//! All tests in this module are marked `#[ignore]` so they do not run with
//! `cargo test` by default.  Run them explicitly with:
//!
//!     cargo test -- --ignored
//!
//! Properties verified here correspond to Properties 1–6 in design.md.

use proptest::prelude::*;

use crate::broker::SIGNAL_RESPONSE;
use crate::command::{parse_command, validate_command, Action, LockCommand};
use crate::process::process_command;
use crate::safety::check_safety;
use crate::testing::MockBrokerClient;

// Helper: build a single-threaded tokio runtime for async property tests.
fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio runtime")
}

fn make_cmd(action: Action) -> LockCommand {
    LockCommand {
        command_id: "prop-test-id".to_string(),
        action,
        doors: vec!["driver".to_string()],
        source: None,
        vin: None,
        timestamp: None,
    }
}

// TS-03-P1: Command Validation Completeness (Property 1)
// Any string either parses+validates correctly or is rejected.
proptest! {
    #[test]
    #[ignore]
    fn proptest_command_validation_completeness(input in ".*") {
        match parse_command(&input) {
            Ok(cmd) => {
                match validate_command(&cmd) {
                    Ok(()) => {
                        // If validation passes, the command must be well-formed.
                        prop_assert!(!cmd.command_id.is_empty(), "valid cmd_id must be non-empty");
                        prop_assert!(
                            cmd.action == Action::Lock || cmd.action == Action::Unlock,
                            "action must be Lock or Unlock"
                        );
                        prop_assert!(
                            cmd.doors.iter().all(|d| d == "driver"),
                            "doors must only contain 'driver'"
                        );
                    }
                    Err(_) => {
                        // Rejected is also acceptable.
                    }
                }
            }
            Err(_) => {
                // Parse failure is acceptable.
            }
        }
    }
}

// TS-03-P2: Safety Gate for Lock (Property 2)
// Lock is allowed iff speed < 1.0 and door closed. Speed check has priority.
proptest! {
    #[test]
    #[ignore]
    fn proptest_safety_gate_lock(speed in 0.0f32..200.0f32, door_open in proptest::bool::ANY) {
        use crate::safety::SafetyResult;
        let runtime = rt();
        let mock = MockBrokerClient::new().with_speed(speed).with_door_open(door_open);
        let result = runtime.block_on(check_safety(&mock));

        if speed < 1.0 && !door_open {
            prop_assert_eq!(result, SafetyResult::Safe);
        } else if speed >= 1.0 {
            prop_assert_eq!(result, SafetyResult::VehicleMoving);
        } else {
            // speed < 1.0 && door_open
            prop_assert_eq!(result, SafetyResult::DoorOpen);
        }
    }
}

// TS-03-P3: Unlock Always Succeeds (Property 3)
proptest! {
    #[test]
    #[ignore]
    fn proptest_unlock_always_succeeds(speed in 0.0f32..200.0f32, door_open in proptest::bool::ANY) {
        let runtime = rt();
        let mock = MockBrokerClient::new().with_speed(speed).with_door_open(door_open);
        let mut lock_state = true;
        let cmd = make_cmd(Action::Unlock);
        let response = runtime.block_on(process_command(&mock, &cmd, &mut lock_state));
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
    }
}

// TS-03-P4: State-Response Consistency (Property 4)
// After a successful command, lock_state matches the requested action.
proptest! {
    #[test]
    #[ignore]
    fn proptest_state_response_consistency(action_is_lock in proptest::bool::ANY) {
        let runtime = rt();
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false);
        let mut lock_state = false;
        let cmd = make_cmd(if action_is_lock { Action::Lock } else { Action::Unlock });
        let response = runtime.block_on(process_command(&mock, &cmd, &mut lock_state));
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
        if action_is_lock {
            prop_assert!(lock_state, "lock_state should be true after lock");
        } else {
            prop_assert!(!lock_state, "lock_state should be false after unlock");
        }
    }
}

// TS-03-P5: Idempotent Operations (Property 5)
// Repeating the same command N times results in at most one set_bool call.
proptest! {
    #[test]
    #[ignore]
    fn proptest_idempotent_operations(action_is_lock in proptest::bool::ANY, n in 1usize..5) {
        let runtime = rt();
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false);
        let mut lock_state = false;
        let cmd = make_cmd(if action_is_lock { Action::Lock } else { Action::Unlock });

        for _ in 0..n {
            let response = runtime.block_on(process_command(&mock, &cmd, &mut lock_state));
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
            prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
        }

        let calls = mock.set_bool_calls();
        prop_assert!(
            calls.len() <= 1,
            "set_bool should be called at most once for {} repetitions",
            n
        );
    }
}

// TS-03-P6: Response Completeness (Property 6)
// Every processed command produces exactly one response with required fields.
proptest! {
    #[test]
    #[ignore]
    fn proptest_response_completeness(
        action_is_lock in proptest::bool::ANY,
        speed in proptest::sample::select(vec![0.0f32, 50.0f32]),
        door_open in proptest::bool::ANY,
    ) {
        let runtime = rt();
        let mock = MockBrokerClient::new().with_speed(speed).with_door_open(door_open);
        let mut lock_state = false;
        let cmd = make_cmd(if action_is_lock { Action::Lock } else { Action::Unlock });

        let response = runtime.block_on(process_command(&mock, &cmd, &mut lock_state));
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        // Required fields present
        prop_assert!(!parsed["command_id"].is_null());
        prop_assert_eq!(parsed["command_id"].as_str().unwrap(), cmd.command_id.as_str());
        let status = parsed["status"].as_str().unwrap();
        prop_assert!(status == "success" || status == "failed");
        prop_assert!(parsed["timestamp"].as_i64().unwrap_or(0) > 0);

        // Exactly one set_string call to SIGNAL_RESPONSE
        let str_calls = mock.set_string_calls();
        let signal_response_calls: Vec<_> = str_calls
            .iter()
            .filter(|(sig, _)| sig == SIGNAL_RESPONSE)
            .collect();
        prop_assert_eq!(
            signal_response_calls.len(),
            1,
            "exactly one response must be published to SIGNAL_RESPONSE"
        );
    }
}
