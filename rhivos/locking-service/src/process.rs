use crate::broker::{BrokerClient, SIGNAL_IS_LOCKED, SIGNAL_RESPONSE};
use crate::command::{Action, LockCommand};
use crate::response::{failure_response, success_response};
use crate::safety::{check_safety, SafetyResult};

/// Process a validated lock/unlock command.
///
/// For lock: checks idempotency first (already locked returns success without
/// state write), then checks safety constraints, updates state if changed.
/// For unlock: skips safety, updates state if changed (idempotent if already
/// unlocked).
///
/// Publishes the response JSON to DATA_BROKER via `set_string`. If the
/// response publish fails, logs the error and continues (03-REQ-5.E1).
///
/// Returns the JSON response string.
pub async fn process_command<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    let response = match cmd.action {
        Action::Lock => process_lock(broker, cmd, lock_state).await,
        Action::Unlock => process_unlock(broker, cmd, lock_state).await,
    };

    // Publish response to DATA_BROKER; log and continue on failure (03-REQ-5.E1).
    if let Err(e) = broker.set_string(SIGNAL_RESPONSE, &response).await {
        tracing::error!(command_id = %cmd.command_id, "failed to publish response: {e}");
    }

    response
}

/// Process a lock command with safety validation and idempotency.
async fn process_lock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Idempotent: already locked -> success without state write (03-REQ-4.E1).
    if *lock_state {
        return success_response(&cmd.command_id);
    }

    // Safety constraint check (03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3).
    match check_safety(broker).await {
        SafetyResult::Safe => {
            // Update lock state on DATA_BROKER (03-REQ-4.1).
            // Only update in-memory lock_state on success to avoid divergence
            // from DATA_BROKER state.
            match broker.set_bool(SIGNAL_IS_LOCKED, true).await {
                Ok(()) => {
                    *lock_state = true;
                    success_response(&cmd.command_id)
                }
                Err(e) => {
                    tracing::error!(command_id = %cmd.command_id, "failed to set lock state: {e}");
                    failure_response(&cmd.command_id, "broker_error")
                }
            }
        }
        SafetyResult::VehicleMoving => failure_response(&cmd.command_id, "vehicle_moving"),
        SafetyResult::DoorOpen => failure_response(&cmd.command_id, "door_open"),
    }
}

/// Process an unlock command (no safety checks, 03-REQ-3.4).
async fn process_unlock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Idempotent: already unlocked -> success without state write (03-REQ-4.E2).
    if !*lock_state {
        return success_response(&cmd.command_id);
    }

    // Update lock state on DATA_BROKER (03-REQ-4.2).
    // Only update in-memory lock_state on success to avoid divergence
    // from DATA_BROKER state.
    match broker.set_bool(SIGNAL_IS_LOCKED, false).await {
        Ok(()) => {
            *lock_state = false;
            success_response(&cmd.command_id)
        }
        Err(e) => {
            tracing::error!(command_id = %cmd.command_id, "failed to set lock state: {e}");
            failure_response(&cmd.command_id, "broker_error")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Action;
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

    // TS-03-11: Verify that a successful lock command sets IsLocked to true on the
    // broker.
    #[tokio::test]
    async fn test_lock_sets_state_true() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        assert!(lock_state, "lock_state should be true after lock");
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls()
                .iter()
                .any(|(s, v)| s == "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked" && *v),
            "set_bool should be called with (SIGNAL_IS_LOCKED, true)"
        );
    }

    // TS-03-12: Verify that an unlock command sets IsLocked to false on the broker.
    #[tokio::test]
    async fn test_unlock_sets_state_false() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        assert!(!lock_state, "lock_state should be false after unlock");
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls()
                .iter()
                .any(|(s, v)| s == "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked" && !v),
            "set_bool should be called with (SIGNAL_IS_LOCKED, false)"
        );
    }

    // TS-03-10: Verify unlock succeeds regardless of speed and door state.
    #[tokio::test]
    async fn test_unlock_bypasses_safety() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(100.0))
            .with_door_open(Some(true));
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        assert_eq!(parsed["status"], "success");
    }

    // TS-03-E8: Verify locking an already-locked door returns success without
    // changing state.
    #[tokio::test]
    async fn test_lock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = true;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "no set_bool calls expected for idempotent lock"
        );
    }

    // TS-03-E9: Verify unlocking an already-unlocked door returns success without
    // changing state.
    #[tokio::test]
    async fn test_unlock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "no set_bool calls expected for idempotent unlock"
        );
    }

    // Review finding: Verify lock_state is NOT updated when set_bool fails during
    // lock, preventing in-memory state divergence from DATA_BROKER (03-REQ-4.1).
    #[tokio::test]
    async fn test_lock_state_unchanged_on_set_bool_failure() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        mock.fail_next_set_bool();
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        // The command should report failure since the state write failed.
        assert_eq!(parsed["status"], "failed", "lock should fail when set_bool fails");
        assert!(
            !lock_state,
            "lock_state must remain false when set_bool fails"
        );
    }

    // Review finding: Verify lock_state is NOT updated when set_bool fails during
    // unlock, preventing in-memory state divergence from DATA_BROKER (03-REQ-4.2).
    #[tokio::test]
    async fn test_unlock_state_unchanged_on_set_bool_failure() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        mock.fail_next_set_bool();
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("valid JSON");
        // The command should report failure since the state write failed.
        assert_eq!(parsed["status"], "failed", "unlock should fail when set_bool fails");
        assert!(
            lock_state,
            "lock_state must remain true when set_bool fails"
        );
    }

    // TS-03-E10: Verify the service continues processing after a response publish
    // failure.
    #[tokio::test]
    async fn test_response_publish_failure() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        mock.fail_next_set_string();
        let mut lock_state = false;
        // First command - response publish fails but command still processes.
        let _response1 = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        // Second command should still succeed.
        let response2 = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response2).expect("valid JSON");
        assert_eq!(parsed["status"], "success");
    }
}
