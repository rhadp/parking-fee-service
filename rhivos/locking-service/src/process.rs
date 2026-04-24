use crate::broker::{BrokerClient, SIGNAL_IS_LOCKED, SIGNAL_RESPONSE};
use crate::command::{Action, LockCommand};
use crate::response::{failure_response, success_response};
use crate::safety::{check_safety, SafetyResult};

/// Process a validated lock/unlock command.
///
/// Dispatches to lock or unlock handling, publishes the response to
/// DATA_BROKER via `SIGNAL_RESPONSE`, and returns the response JSON string.
/// If the response publish fails, the error is logged and processing continues
/// (03-REQ-5.E1).
pub async fn process_command<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    let response = match cmd.action {
        Action::Lock => process_lock(broker, cmd, lock_state).await,
        Action::Unlock => process_unlock(broker, cmd, lock_state).await,
    };

    // Publish response to DATA_BROKER; log and continue on failure (03-REQ-5.E1)
    if let Err(e) = broker.set_string(SIGNAL_RESPONSE, &response).await {
        eprintln!("failed to publish response for command {}: {e}", cmd.command_id);
    }

    response
}

/// Process a lock command: check safety constraints, then update state if safe.
async fn process_lock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Check safety constraints (03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3)
    match check_safety(broker).await {
        SafetyResult::Safe => {}
        SafetyResult::VehicleMoving => {
            return failure_response(&cmd.command_id, "vehicle_moving");
        }
        SafetyResult::DoorOpen => {
            return failure_response(&cmd.command_id, "door_open");
        }
    }

    // Idempotent: skip state write if already locked (03-REQ-4.E1)
    if !*lock_state {
        if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, true).await {
            eprintln!("failed to set lock state: {e}");
        }
        *lock_state = true;
    }

    success_response(&cmd.command_id)
}

/// Process an unlock command: update state without safety checks (03-REQ-3.4).
async fn process_unlock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Idempotent: skip state write if already unlocked (03-REQ-4.E2)
    if *lock_state {
        if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, false).await {
            eprintln!("failed to set lock state: {e}");
        }
        *lock_state = false;
    }

    success_response(&cmd.command_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::SIGNAL_IS_LOCKED;
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

    // TS-03-11: lock sets IsLocked = true
    #[tokio::test]
    async fn test_lock_sets_state_true() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0_f32)
            .with_door_open(false);
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        assert!(lock_state, "lock_state should be true after lock command");
        let calls = mock.set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(sig, val)| sig == SIGNAL_IS_LOCKED && *val),
            "should have called set_bool with IsLocked=true"
        );
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("should be valid JSON");
        assert_eq!(parsed["status"], "success");
    }

    // TS-03-12: unlock sets IsLocked = false
    #[tokio::test]
    async fn test_unlock_sets_state_false() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0_f32)
            .with_door_open(false)
            .with_locked(true);
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        assert!(
            !lock_state,
            "lock_state should be false after unlock command"
        );
        let calls = mock.set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(sig, val)| sig == SIGNAL_IS_LOCKED && !*val),
            "should have called set_bool with IsLocked=false"
        );
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("should be valid JSON");
        assert_eq!(parsed["status"], "success");
    }

    // TS-03-10: unlock succeeds regardless of speed and door state
    #[tokio::test]
    async fn test_unlock_bypasses_safety() {
        let mock = MockBrokerClient::new()
            .with_speed(100.0_f32)
            .with_door_open(true)
            .with_locked(true);
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("should be valid JSON");
        assert_eq!(parsed["status"], "success");
    }

    // TS-03-E8: lock already-locked returns success, no state write
    #[tokio::test]
    async fn test_lock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0_f32)
            .with_door_open(false)
            .with_locked(true);
        let mut lock_state = true;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("should be valid JSON");
        assert_eq!(parsed["status"], "success");
        assert_eq!(
            mock.set_bool_calls().len(),
            0,
            "should not call set_bool when already locked"
        );
    }

    // TS-03-E9: unlock already-unlocked returns success, no state write
    #[tokio::test]
    async fn test_unlock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0_f32)
            .with_door_open(false);
        let mut lock_state = false;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("should be valid JSON");
        assert_eq!(parsed["status"], "success");
        assert_eq!(
            mock.set_bool_calls().len(),
            0,
            "should not call set_bool when already unlocked"
        );
    }

    // TS-03-E10: service continues after response publish failure
    #[tokio::test]
    async fn test_response_publish_failure() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0_f32)
            .with_door_open(false);
        mock.fail_next_set_string();
        let mut lock_state = false;
        // First command -- response publish fails
        let _response1 = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        // Second command -- should still work
        let response2 = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response2).expect("should be valid JSON");
        assert_eq!(parsed["status"], "success");
    }
}
