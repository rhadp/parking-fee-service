//! Command processing: dispatches lock/unlock, checks safety, manages state, publishes response.
use crate::broker::{BrokerClient, SIGNAL_IS_LOCKED, SIGNAL_RESPONSE};
use crate::command::{Action, LockCommand};
use crate::response::{failure_response, success_response};
use crate::safety::{check_safety, SafetyResult};

/// Process a validated lock/unlock command.
///
/// Design decision: idempotent check is performed BEFORE safety validation.
/// If the door is already in the requested state, return success immediately
/// without running safety constraints (ASIL-B: allow "keep locked" even when unsafe).
///
/// Returns the response JSON string. Publish failures are logged and do not
/// prevent processing subsequent commands.
pub async fn process_command<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    let id = &cmd.command_id;

    // Idempotent check BEFORE safety (design decision per ASIL-B requirements).
    let already_matches = match cmd.action {
        Action::Lock => *lock_state,
        Action::Unlock => !*lock_state,
    };

    let response = if already_matches {
        success_response(id)
    } else {
        match cmd.action {
            Action::Lock => {
                let safety = check_safety(broker).await;
                match safety {
                    SafetyResult::Safe => {
                        if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, true).await {
                            tracing::error!("Failed to set IsLocked=true: {e}");
                        }
                        *lock_state = true;
                        success_response(id)
                    }
                    SafetyResult::VehicleMoving => failure_response(id, "vehicle_moving"),
                    SafetyResult::DoorOpen => failure_response(id, "door_open"),
                }
            }
            Action::Unlock => {
                if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, false).await {
                    tracing::error!("Failed to set IsLocked=false: {e}");
                }
                *lock_state = false;
                success_response(id)
            }
        }
    };

    // Always publish response; log errors but don't fail.
    if let Err(e) = broker.set_string(SIGNAL_RESPONSE, &response).await {
        tracing::error!("Failed to publish response: {e}");
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Action, LockCommand};
    use crate::testing::MockBrokerClient;

    fn make_lock_cmd(id: &str) -> LockCommand {
        LockCommand {
            command_id: id.to_string(),
            action: Action::Lock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    fn make_unlock_cmd(id: &str) -> LockCommand {
        LockCommand {
            command_id: id.to_string(),
            action: Action::Unlock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    // TS-03-11: Lock command sets IsLocked = true on broker; response is success
    #[tokio::test]
    async fn test_lock_sets_state_true() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(false);
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd("id-1"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(lock_state, "lock_state must be true after lock");
        let calls = mock.set_bool_calls();
        assert!(
            calls.contains(&(SIGNAL_IS_LOCKED.to_string(), true)),
            "set_bool must be called with IsLocked=true"
        );
    }

    // TS-03-12: Unlock command sets IsLocked = false on broker; response is success
    #[tokio::test]
    async fn test_unlock_sets_state_false() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(false)
            .with_locked(true);
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd("id-2"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(!lock_state, "lock_state must be false after unlock");
        let calls = mock.set_bool_calls();
        assert!(
            calls.contains(&(SIGNAL_IS_LOCKED.to_string(), false)),
            "set_bool must be called with IsLocked=false"
        );
    }

    // TS-03-10: Unlock bypasses safety — succeeds regardless of speed/door
    #[tokio::test]
    async fn test_unlock_bypasses_safety() {
        let mock = MockBrokerClient::new()
            .with_speed(100.0)
            .with_door_open(true)
            .with_locked(true);
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd("id-3"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success", "unlock must always succeed");
    }

    // TS-03-E8: Lock when already locked returns success without state write
    #[tokio::test]
    async fn test_lock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(false)
            .with_locked(true);
        let mut lock_state = true; // already locked
        let response = process_command(&mock, &make_lock_cmd("id-4"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "set_bool must not be called when already locked"
        );
    }

    // TS-03-E9: Unlock when already unlocked returns success without state write
    #[tokio::test]
    async fn test_unlock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(false);
        let mut lock_state = false; // already unlocked
        let response = process_command(&mock, &make_unlock_cmd("id-5"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "success");
        assert!(
            mock.set_bool_calls().is_empty(),
            "set_bool must not be called when already unlocked"
        );
    }

    // TS-03-E10: Service continues after response publish failure
    #[tokio::test]
    async fn test_response_publish_failure() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(false);
        mock.fail_next_set_string();
        let mut lock_state = false;
        // First command: set_string fails (response publish failure)
        let _response1 = process_command(&mock, &make_lock_cmd("id-6"), &mut lock_state).await;
        // Second command: must succeed
        let response2 = process_command(&mock, &make_unlock_cmd("id-7"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response2).unwrap();
        assert_eq!(
            parsed["status"], "success",
            "second command must succeed after publish failure"
        );
    }

    // Addresses major finding: idempotent check must short-circuit BEFORE safety validation.
    // When door is already locked AND vehicle is moving, idempotent check wins → success.
    #[tokio::test]
    async fn test_lock_idempotent_with_safety_violation() {
        let mock = MockBrokerClient::new()
            .with_speed(50.0)   // unsafe: vehicle moving
            .with_door_open(false)
            .with_locked(true); // already locked
        let mut lock_state = true;
        let response = process_command(&mock, &make_lock_cmd("id-8"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        // Design decision: idempotent check first → success (no safety check needed)
        assert_eq!(
            parsed["status"], "success",
            "idempotent check must short-circuit before safety validation"
        );
        assert!(
            mock.set_bool_calls().is_empty(),
            "no state write when already in requested state"
        );
    }

    // Lock rejected when vehicle moving (safety check applies when not idempotent)
    #[tokio::test]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(50.0)
            .with_door_open(false);
        let mut lock_state = false; // not locked, so idempotent check doesn't trigger
        let response = process_command(&mock, &make_lock_cmd("id-9"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
    }

    // Lock rejected when door open
    #[tokio::test]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(true);
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd("id-10"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "door_open");
    }

    // Response must include command_id matching the request
    #[tokio::test]
    async fn test_response_echoes_command_id() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(false);
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd("echo-test-id"), &mut lock_state).await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["command_id"], "echo-test-id");
    }

    // Exactly one set_string call to SIGNAL_RESPONSE per processed command
    #[tokio::test]
    async fn test_exactly_one_response_published() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(false);
        let mut lock_state = false;
        let _response = process_command(&mock, &make_lock_cmd("id-pub"), &mut lock_state).await;
        let string_calls = mock.set_string_calls();
        let response_calls: Vec<_> = string_calls
            .iter()
            .filter(|(sig, _)| sig == SIGNAL_RESPONSE)
            .collect();
        assert_eq!(
            response_calls.len(),
            1,
            "exactly one response must be published"
        );
    }
}
