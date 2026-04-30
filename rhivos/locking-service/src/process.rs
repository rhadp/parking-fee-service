use crate::broker::{BrokerClient, SIGNAL_IS_LOCKED, SIGNAL_RESPONSE};
use crate::command::{Action, LockCommand};
use crate::response::{failure_response, success_response};
use crate::safety::{check_safety, SafetyResult};

/// Process a validated lock/unlock command.
///
/// For lock commands: checks safety constraints, updates lock state if safe.
/// For unlock commands: always succeeds, no safety check.
/// Handles idempotent operations (skip `set_bool` if state already matches).
/// Publishes response via `set_string` to the response signal.
///
/// Returns the response JSON string.
pub async fn process_command<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    let response = match cmd.action {
        Action::Lock => process_lock(broker, cmd, lock_state).await,
        Action::Unlock => process_unlock(broker, cmd, lock_state).await,
    };

    // Publish response; log and continue on failure (03-REQ-5.E1).
    if let Err(e) = broker.set_string(SIGNAL_RESPONSE, &response).await {
        tracing::error!(
            command_id = %cmd.command_id,
            "failed to publish response: {e}"
        );
    }

    response
}

/// Process a lock command with safety validation.
async fn process_lock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Check safety constraints (03-REQ-3.1, 03-REQ-3.2).
    let safety = check_safety(broker).await;
    match safety {
        SafetyResult::VehicleMoving => {
            tracing::warn!(
                command_id = %cmd.command_id,
                "lock rejected: vehicle moving"
            );
            return failure_response(&cmd.command_id, "vehicle_moving");
        }
        SafetyResult::DoorOpen => {
            tracing::warn!(
                command_id = %cmd.command_id,
                "lock rejected: door open"
            );
            return failure_response(&cmd.command_id, "door_open");
        }
        SafetyResult::Safe => {}
    }

    // Idempotent: skip set_bool if already locked (03-REQ-4.E1).
    if !*lock_state {
        if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, true).await {
            tracing::error!(
                command_id = %cmd.command_id,
                "failed to set lock state: {e}"
            );
        }
        *lock_state = true;
        tracing::info!(command_id = %cmd.command_id, "door locked");
    } else {
        tracing::info!(
            command_id = %cmd.command_id,
            "door already locked (idempotent)"
        );
    }

    success_response(&cmd.command_id)
}

/// Process an unlock command (no safety check - 03-REQ-3.4).
async fn process_unlock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Idempotent: skip set_bool if already unlocked (03-REQ-4.E2).
    if *lock_state {
        if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, false).await {
            tracing::error!(
                command_id = %cmd.command_id,
                "failed to set lock state: {e}"
            );
        }
        *lock_state = false;
        tracing::info!(command_id = %cmd.command_id, "door unlocked");
    } else {
        tracing::info!(
            command_id = %cmd.command_id,
            "door already unlocked (idempotent)"
        );
    }

    success_response(&cmd.command_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Action, LockCommand};
    use crate::testing::MockBrokerClient;

    fn make_lock_cmd() -> LockCommand {
        LockCommand {
            command_id: "test-cmd-1".to_string(),
            action: Action::Lock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    fn make_unlock_cmd() -> LockCommand {
        LockCommand {
            command_id: "test-cmd-2".to_string(),
            action: Action::Unlock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    // TS-03-11: Verify that a successful lock command sets IsLocked to true.
    #[tokio::test]
    async fn test_lock_sets_state_true() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        assert!(lock_state, "lock_state should be true after lock");
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        let calls = mock.set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(sig, val)| sig == "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
                    && *val),
            "set_bool should be called with IsLocked=true"
        );
    }

    // TS-03-12: Verify that an unlock command sets IsLocked to false.
    #[tokio::test]
    async fn test_unlock_sets_state_false() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false))
            .with_locked(Some(true));
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        assert!(!lock_state, "lock_state should be false after unlock");
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        let calls = mock.set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(sig, val)| sig == "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
                    && !*val),
            "set_bool should be called with IsLocked=false"
        );
    }

    // TS-03-10: Verify unlock succeeds regardless of speed and door state.
    #[tokio::test]
    async fn test_unlock_bypasses_safety() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(100.0))
            .with_door_open(Some(true))
            .with_locked(Some(true));
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
    }

    // TS-03-E8: Verify locking an already-locked door returns success, no state write.
    #[tokio::test]
    async fn test_lock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false))
            .with_locked(Some(true));
        let mut lock_state = true;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "set_bool should not be called when already locked"
        );
    }

    // TS-03-E9: Verify unlocking an already-unlocked door returns success, no state write.
    #[tokio::test]
    async fn test_unlock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "set_bool should not be called when already unlocked"
        );
    }

    // TS-03-E10: Verify the service continues after a response publish failure.
    #[tokio::test]
    async fn test_response_publish_failure() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        mock.fail_next_set_string();
        let mut lock_state = false;

        // First command: response publish will fail.
        let _response1 = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;

        // Second command should still work.
        let response2 = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response2).unwrap();
        assert_eq!(parsed["status"], "success");
    }
}
