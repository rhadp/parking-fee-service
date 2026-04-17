//! Command processing orchestration.
//!
//! `process_command` dispatches a validated command to either the lock or
//! unlock path, handles idempotent state checks, publishes the response via
//! DATA_BROKER, and returns the response JSON string.

#![allow(dead_code)]

use crate::broker::BrokerClient;
use crate::command::LockCommand;

/// Process a validated lock/unlock command.
///
/// - Lock: runs safety checks, updates state only when needed (idempotent).
/// - Unlock: skips safety checks, updates state only when needed (idempotent).
///
/// Always publishes a response to `SIGNAL_RESPONSE`. On publish failure the
/// error is logged and the function returns the response JSON regardless
/// (03-REQ-5.E1).
///
/// Returns the JSON response string (for testing / further processing).
pub async fn process_command<B: BrokerClient>(
    _broker: &B,
    _cmd: &LockCommand,
    _lock_state: &mut bool,
) -> String {
    todo!("process_command — implemented in task group 3")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::SIGNAL_IS_LOCKED;
    use crate::command::Action;
    use crate::testing::MockBrokerClient;

    fn make_lock_cmd() -> LockCommand {
        LockCommand {
            command_id: "test-lock-id".to_string(),
            action: Action::Lock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    fn make_unlock_cmd() -> LockCommand {
        LockCommand {
            command_id: "test-unlock-id".to_string(),
            action: Action::Unlock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    /// TS-03-11 / 03-REQ-4.1: Successful lock sets IsLocked = true and returns success.
    #[tokio::test]
    async fn test_lock_sets_state_true() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;

        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["status"], "success");
        assert!(lock_state, "lock_state must be true after lock");
        assert!(
            mock.set_bool_calls()
                .iter()
                .any(|(sig, val)| sig == SIGNAL_IS_LOCKED && *val),
            "set_bool must be called with IsLocked=true"
        );
    }

    /// TS-03-12 / 03-REQ-4.2: Unlock sets IsLocked = false and returns success.
    #[tokio::test]
    async fn test_unlock_sets_state_false() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = true;

        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["status"], "success");
        assert!(!lock_state, "lock_state must be false after unlock");
        assert!(
            mock.set_bool_calls()
                .iter()
                .any(|(sig, val)| sig == SIGNAL_IS_LOCKED && !val),
            "set_bool must be called with IsLocked=false"
        );
    }

    /// TS-03-10 / 03-REQ-3.4: Unlock bypasses safety checks (high speed + door open).
    #[tokio::test]
    async fn test_unlock_bypasses_safety() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(100.0))
            .with_door_open(Some(true));
        let mut lock_state = true;

        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["status"], "success", "unlock must succeed regardless of safety state");
    }

    /// TS-03-E8 / 03-REQ-4.E1: Locking an already-locked door returns success with no state write.
    #[tokio::test]
    async fn test_lock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = true; // already locked

        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "set_bool must NOT be called when door is already locked"
        );
    }

    /// TS-03-E9 / 03-REQ-4.E2: Unlocking an already-unlocked door returns success with no state write.
    #[tokio::test]
    async fn test_unlock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false; // already unlocked

        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "set_bool must NOT be called when door is already unlocked"
        );
    }

    /// TS-03-E10 / 03-REQ-5.E1: Service continues processing after response publish failure.
    #[tokio::test]
    async fn test_response_publish_failure() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        mock.fail_next_set_string();

        let mut lock_state = false;
        // First command: response publish will fail
        process_command(&mock, &make_lock_cmd(), &mut lock_state).await;

        // Second command: must succeed normally (service continued)
        let response2 = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response2).unwrap();
        assert_eq!(parsed["status"], "success", "second command must succeed after publish failure");
    }

    /// Lock rejected when vehicle is moving returns "vehicle_moving" status.
    #[tokio::test]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(30.0))
            .with_door_open(Some(false));
        let mut lock_state = false;

        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
        assert!(!lock_state, "lock_state must not change when command is rejected");
    }

    /// Lock rejected when door is open returns "door_open" status.
    #[tokio::test]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(true));
        let mut lock_state = false;

        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "door_open");
        assert!(!lock_state, "lock_state must not change when command is rejected");
    }
}
