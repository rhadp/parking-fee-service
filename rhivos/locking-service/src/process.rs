//! Command processing orchestration.
//!
//! `process_command` dispatches to lock or unlock handling, enforces safety
//! constraints for lock, manages idempotent state, and publishes responses.

use crate::broker::{BrokerClient, SIGNAL_IS_LOCKED, SIGNAL_RESPONSE};
use crate::command::{Action, LockCommand};
use crate::response::{failure_response, success_response};
use crate::safety::{check_safety, SafetyResult};

/// Process a validated `LockCommand`.
///
/// - For `Lock`: runs safety checks, updates lock state, publishes response.
/// - For `Unlock`: skips safety checks, updates lock state, publishes response.
/// - Idempotent: if state already matches the requested action, returns "success"
///   without writing to the lock state signal.
/// - On response-publish failure: logs the error and returns the response JSON
///   so the caller can continue (03-REQ-5.E1).
///
/// Returns the JSON response string.
pub async fn process_command<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    let response_json = match cmd.action {
        Action::Lock => process_lock(broker, cmd, lock_state).await,
        Action::Unlock => process_unlock(broker, cmd, lock_state).await,
    };

    // Publish response to DATA_BROKER; log and continue on failure (03-REQ-5.E1).
    if let Err(e) = broker.set_string(SIGNAL_RESPONSE, &response_json).await {
        tracing::error!(
            command_id = %cmd.command_id,
            error = ?e,
            "Failed to publish command response to DATA_BROKER"
        );
    }

    response_json
}

/// Handle a lock action.
///
/// Checks idempotency first, then runs safety validation before updating state.
async fn process_lock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Idempotent: already locked — return success without a state write (03-REQ-4.E1).
    if *lock_state {
        return success_response(&cmd.command_id);
    }

    // Safety check required for lock commands (03-REQ-3.1, 03-REQ-3.2).
    match check_safety(broker).await {
        SafetyResult::Safe => {
            // Write lock state to DATA_BROKER (03-REQ-4.1).
            if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, true).await {
                tracing::error!(
                    command_id = %cmd.command_id,
                    error = ?e,
                    "Failed to write IsLocked=true to DATA_BROKER"
                );
            }
            *lock_state = true;
            success_response(&cmd.command_id)
        }
        SafetyResult::VehicleMoving => failure_response(&cmd.command_id, "vehicle_moving"),
        SafetyResult::DoorOpen => failure_response(&cmd.command_id, "door_open"),
    }
}

/// Handle an unlock action.
///
/// Safety constraints are NOT checked for unlock commands (03-REQ-3.4).
async fn process_unlock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Idempotent: already unlocked — return success without a state write (03-REQ-4.E2).
    if !*lock_state {
        return success_response(&cmd.command_id);
    }

    // Write lock state to DATA_BROKER (03-REQ-4.2).
    if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, false).await {
        tracing::error!(
            command_id = %cmd.command_id,
            error = ?e,
            "Failed to write IsLocked=false to DATA_BROKER"
        );
    }
    *lock_state = false;
    success_response(&cmd.command_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::SIGNAL_IS_LOCKED;
    use crate::command::{Action, LockCommand};
    use crate::testing::MockBrokerClient;

    fn make_lock_cmd() -> LockCommand {
        LockCommand {
            command_id: "test-lock-001".to_string(),
            action: Action::Lock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    fn make_unlock_cmd() -> LockCommand {
        LockCommand {
            command_id: "test-unlock-001".to_string(),
            action: Action::Unlock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    // TS-03-11: Lock sets IsLocked = true and returns "success"
    #[tokio::test(flavor = "current_thread")]
    async fn test_lock_sets_state_true() {
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false);
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(lock_state, "lock_state should be true after lock command");
        assert!(
            mock.set_bool_calls()
                .contains(&(SIGNAL_IS_LOCKED.to_string(), true)),
            "set_bool should have been called with IsLocked=true"
        );
    }

    // TS-03-12: Unlock sets IsLocked = false and returns "success"
    #[tokio::test(flavor = "current_thread")]
    async fn test_unlock_sets_state_false() {
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false);
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(!lock_state, "lock_state should be false after unlock command");
        assert!(
            mock.set_bool_calls()
                .contains(&(SIGNAL_IS_LOCKED.to_string(), false)),
            "set_bool should have been called with IsLocked=false"
        );
    }

    // TS-03-10: Unlock bypasses safety (high speed and door open → still succeeds)
    #[tokio::test(flavor = "current_thread")]
    async fn test_unlock_bypasses_safety() {
        let mock = MockBrokerClient::new()
            .with_speed(100.0)
            .with_door_open(true);
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            parsed["status"], "success",
            "unlock should always succeed regardless of speed/door state"
        );
    }

    // TS-03-E8: Locking an already-locked door returns success without state write
    #[tokio::test(flavor = "current_thread")]
    async fn test_lock_idempotent() {
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false);
        let mut lock_state = true; // already locked
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "set_bool should NOT be called when door is already locked"
        );
    }

    // TS-03-E9: Unlocking an already-unlocked door returns success without state write
    #[tokio::test(flavor = "current_thread")]
    async fn test_unlock_idempotent() {
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false);
        let mut lock_state = false; // already unlocked
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "set_bool should NOT be called when door is already unlocked"
        );
    }

    // TS-03-E10: Service continues processing after response publish failure
    #[tokio::test(flavor = "current_thread")]
    async fn test_response_publish_failure() {
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false);
        let mut lock_state = false;

        // First command: response publish fails
        mock.fail_next_set_string();
        process_command(&mock, &make_lock_cmd(), &mut lock_state).await;

        // Second command: should succeed
        let response2 = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response2).unwrap();
        assert_eq!(
            parsed["status"], "success",
            "second command should succeed after prior publish failure"
        );
    }

    // Additional: lock rejected when moving returns "failed" + "vehicle_moving"
    #[tokio::test(flavor = "current_thread")]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new().with_speed(50.0).with_door_open(false);
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
        assert!(
            !lock_state,
            "lock_state should remain false after rejected lock"
        );
    }

    // Additional: lock rejected when door open returns "failed" + "door_open"
    #[tokio::test(flavor = "current_thread")]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(true);
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "door_open");
    }

    // Additional: response echoes correct command_id
    #[tokio::test(flavor = "current_thread")]
    async fn test_response_echoes_command_id() {
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false);
        let mut lock_state = false;
        let cmd = make_lock_cmd();
        let response = process_command(&mock, &cmd, &mut lock_state).await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["command_id"], cmd.command_id);
    }
}
