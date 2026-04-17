//! Property-based tests for the locking-service.
//!
//! All tests are `#[ignore]` and run separately via:
//!   cargo test -- --ignored
//!
//! Tests use proptest to verify invariants from design.md Properties 1–6.
//! Async operations are run via a single-threaded tokio runtime so that
//! `MockBrokerClient` (which uses `RefCell`) works without `Send` bounds.

use proptest::prelude::*;

use crate::command::{parse_command, validate_command, Action, LockCommand};
use crate::process::process_command;
use crate::safety::{check_safety, SafetyResult};
use crate::testing::MockBrokerClient;

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

fn current_thread_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime for property tests")
}

// ── TS-03-P1: Command Validation Completeness ─────────────────────────────────

/// Property 1: Any string input either produces a valid LockCommand (with
/// non-empty command_id, valid action, doors containing "driver") or is rejected.
#[test]
#[ignore]
fn proptest_command_validation_completeness() {
    proptest!(|(input in ".*")| {
        match parse_command(&input) {
            Ok(cmd) => match validate_command(&cmd) {
                Ok(()) => {
                    prop_assert!(!cmd.command_id.is_empty(), "valid command_id must be non-empty");
                    prop_assert!(
                        cmd.action == Action::Lock || cmd.action == Action::Unlock,
                        "action must be Lock or Unlock"
                    );
                    prop_assert!(
                        cmd.doors.contains(&"driver".to_string()),
                        "doors must contain 'driver'"
                    );
                }
                Err(_) => {} // rejected — acceptable
            },
            Err(_) => {} // rejected — acceptable
        }
    });
}

// ── TS-03-P2: Safety Gate for Lock ────────────────────────────────────────────

/// Property 2: check_safety result is uniquely determined by speed and door state.
/// Speed is evaluated first: if >= 1.0 → VehicleMoving regardless of door state.
#[test]
#[ignore]
fn proptest_safety_gate_lock() {
    let rt = current_thread_rt();
    proptest!(|(speed in 0.0f32..200.0, door_open in proptest::bool::ANY)| {
        let result = rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open));
            check_safety(&mock).await
        });
        if speed < 1.0 && !door_open {
            prop_assert_eq!(result, SafetyResult::Safe);
        } else if speed >= 1.0 {
            prop_assert_eq!(result, SafetyResult::VehicleMoving);
        } else {
            prop_assert_eq!(result, SafetyResult::DoorOpen);
        }
    });
}

// ── TS-03-P3: Unlock Always Succeeds ─────────────────────────────────────────

/// Property 3: An unlock command always succeeds regardless of vehicle speed
/// and door state (03-REQ-3.4).
#[test]
#[ignore]
fn proptest_unlock_always_succeeds() {
    let rt = current_thread_rt();
    proptest!(|(speed in 0.0f32..200.0, door_open in proptest::bool::ANY)| {
        let response = rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open));
            let mut lock_state = true;
            process_command(&mock, &make_cmd(Action::Unlock), &mut lock_state).await
        });
        let parsed: serde_json::Value = serde_json::from_str(&response)
            .expect("process_command must return valid JSON");
        prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
    });
}

// ── TS-03-P4: State-Response Consistency ─────────────────────────────────────

/// Property 4: After a successful command with safe conditions, lock_state
/// matches the requested action (03-REQ-4.1, 03-REQ-4.2, 03-REQ-5.1).
#[test]
#[ignore]
fn proptest_state_response_consistency() {
    let rt = current_thread_rt();
    proptest!(|(action_is_lock in proptest::bool::ANY)| {
        let (response, lock_state) = rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(0.0))
                .with_door_open(Some(false));
            let mut lock_state = false;
            let action = if action_is_lock { Action::Lock } else { Action::Unlock };
            let r = process_command(&mock, &make_cmd(action), &mut lock_state).await;
            (r, lock_state)
        });
        let parsed: serde_json::Value = serde_json::from_str(&response)
            .expect("process_command must return valid JSON");
        prop_assert_eq!(parsed["status"].as_str().unwrap(), "success");
        if action_is_lock {
            prop_assert!(lock_state, "lock_state must be true after lock");
        } else {
            prop_assert!(!lock_state, "lock_state must be false after unlock");
        }
    });
}

// ── TS-03-P5: Idempotent Operations ─────────────────────────────────────────

/// Property 5: Repeating the same command N times yields "success" every time
/// and `set_bool` is called at most once (03-REQ-4.E1, 03-REQ-4.E2).
#[test]
#[ignore]
fn proptest_idempotent_operations() {
    let rt = current_thread_rt();
    proptest!(|(action_is_lock in proptest::bool::ANY, n in 1usize..5)| {
        let set_bool_count = rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(0.0))
                .with_door_open(Some(false));
            let mut lock_state = false;
            let action = if action_is_lock { Action::Lock } else { Action::Unlock };
            for _ in 0..n {
                let r = process_command(&mock, &make_cmd(action.clone()), &mut lock_state).await;
                let parsed: serde_json::Value = serde_json::from_str(&r).unwrap();
                assert_eq!(
                    parsed["status"].as_str().unwrap(),
                    "success",
                    "all repeated commands must succeed"
                );
            }
            mock.set_bool_calls().len()
        });
        prop_assert!(
            set_bool_count <= 1,
            "set_bool called {} times (max allowed: 1)",
            set_bool_count
        );
    });
}

// ── TS-03-P6: Response Completeness ──────────────────────────────────────────

/// Property 6: Every processed command produces exactly one set_string call to
/// SIGNAL_RESPONSE with a JSON body containing command_id, status, and timestamp
/// (03-REQ-5.1, 03-REQ-5.2, 03-REQ-5.3).
#[test]
#[ignore]
fn proptest_response_completeness() {
    let rt = current_thread_rt();
    proptest!(|(
        action_is_lock in proptest::bool::ANY,
        speed in proptest::sample::select(vec![0.0f32, 50.0]),
        door_open in proptest::bool::ANY
    )| {
        let (response, set_string_count) = rt.block_on(async {
            let mock = MockBrokerClient::new()
                .with_speed(Some(speed))
                .with_door_open(Some(door_open));
            let mut lock_state = false;
            let action = if action_is_lock { Action::Lock } else { Action::Unlock };
            let r = process_command(&mock, &make_cmd(action), &mut lock_state).await;
            let count = mock.set_string_calls().len();
            (r, count)
        });
        let parsed: serde_json::Value = serde_json::from_str(&response)
            .expect("process_command must return valid JSON");
        prop_assert_eq!(
            parsed["command_id"].as_str().unwrap(),
            "prop-test-id",
            "command_id must be echoed from request"
        );
        let status = parsed["status"].as_str().unwrap();
        prop_assert!(
            status == "success" || status == "failed",
            "status must be 'success' or 'failed'"
        );
        prop_assert!(
            parsed["timestamp"].as_i64().unwrap_or(0) > 0,
            "timestamp must be a positive integer"
        );
        prop_assert_eq!(set_string_count, 1, "exactly one response must be published");
    });
}
