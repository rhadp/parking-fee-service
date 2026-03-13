use crate::broker::BrokerClient;
use crate::command::{Action, LockCommand};
use crate::response::{failure_response, success_response};
use crate::safety::{check_safety, SafetyResult};

/// VSS signal paths for lock state and response
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Process a validated lock/unlock command.
///
/// Orchestrates: safety check (lock only) → state update → response publish.
/// Returns the response JSON string.
/// If the response publish fails, logs the error and returns the response anyway.
pub async fn process_command<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    let response_json = match cmd.action {
        Action::Lock => process_lock(broker, cmd, lock_state).await,
        Action::Unlock => process_unlock(broker, cmd, lock_state).await,
    };

    // Publish response; log failure but do not abort (03-REQ-5.E1)
    if let Err(e) = broker.set_string(SIGNAL_RESPONSE, &response_json).await {
        tracing::error!("Failed to publish command response: {}", e);
    }

    response_json
}

async fn process_lock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Check safety constraints (03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3)
    match check_safety(broker).await {
        SafetyResult::VehicleMoving => {
            return failure_response(&cmd.command_id, "vehicle_moving");
        }
        SafetyResult::DoorOpen => {
            return failure_response(&cmd.command_id, "door_open");
        }
        SafetyResult::Safe => {}
    }

    // Idempotent: already locked → success with no state change (03-REQ-4.E1)
    if !*lock_state {
        if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, true).await {
            tracing::error!("Failed to set lock state: {}", e);
        } else {
            *lock_state = true;
        }
    }

    success_response(&cmd.command_id)
}

async fn process_unlock<B: BrokerClient>(
    broker: &B,
    cmd: &LockCommand,
    lock_state: &mut bool,
) -> String {
    // Unlock bypasses safety checks (03-REQ-3.4)
    // Idempotent: already unlocked → success with no state change (03-REQ-4.E2)
    if *lock_state {
        if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, false).await {
            tracing::error!("Failed to set lock state: {}", e);
        } else {
            *lock_state = false;
        }
    }

    success_response(&cmd.command_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Action;
    use crate::testing::MockBrokerClient;

    fn make_lock_cmd() -> LockCommand {
        LockCommand {
            command_id: "test-lock-1".to_string(),
            action: Action::Lock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    fn make_unlock_cmd() -> LockCommand {
        LockCommand {
            command_id: "test-unlock-1".to_string(),
            action: Action::Unlock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        }
    }

    // TS-03-11: Lock Sets IsLocked True
    #[tokio::test]
    async fn test_lock_sets_state_true() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("response should be valid JSON");
        assert_eq!(parsed["status"], "success");
        // Check that set_bool was called with IsLocked = true
        let calls = mock.set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(s, v)| s == SIGNAL_IS_LOCKED && *v),
            "should set IsLocked to true"
        );
        assert!(lock_state, "lock_state should be true");
    }

    // TS-03-12: Unlock Sets IsLocked False
    #[tokio::test]
    async fn test_unlock_sets_state_false() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false))
            .with_locked(true);
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("response should be valid JSON");
        assert_eq!(parsed["status"], "success");
        let calls = mock.set_bool_calls();
        assert!(
            calls
                .iter()
                .any(|(s, v)| s == SIGNAL_IS_LOCKED && !*v),
            "should set IsLocked to false"
        );
        assert!(!lock_state, "lock_state should be false");
    }

    // TS-03-10: Unlock Bypasses Safety
    #[tokio::test]
    async fn test_unlock_bypasses_safety() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(100.0))
            .with_door_open(Some(true))
            .with_locked(true);
        let mut lock_state = true;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("response should be valid JSON");
        assert_eq!(parsed["status"], "success");
    }

    // TS-03-E8: Lock When Already Locked (idempotent)
    #[tokio::test]
    async fn test_lock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false))
            .with_locked(true);
        let mut lock_state = true;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("response should be valid JSON");
        assert_eq!(parsed["status"], "success");
        // No set_bool calls — state already matches
        let calls = mock.set_bool_calls();
        assert_eq!(calls.len(), 0, "should not change state when already locked");
    }

    // TS-03-E9: Unlock When Already Unlocked (idempotent)
    #[tokio::test]
    async fn test_unlock_idempotent() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;
        let response = process_command(&mock, &make_unlock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("response should be valid JSON");
        assert_eq!(parsed["status"], "success");
        let calls = mock.set_bool_calls();
        assert_eq!(
            calls.len(),
            0,
            "should not change state when already unlocked"
        );
    }

    // TS-03-E10: Response Publish Failure
    #[tokio::test]
    async fn test_response_publish_failure() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let mut lock_state = false;

        // First command: response publish will fail
        mock.fail_next_set_string();
        let _response1 = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;

        // Second command: should still process normally
        let cmd2 = LockCommand {
            command_id: "test-lock-2".to_string(),
            action: Action::Unlock,
            doors: vec!["driver".to_string()],
            source: None,
            vin: None,
            timestamp: None,
        };
        let response2 = process_command(&mock, &cmd2, &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response2).expect("second response should be valid JSON");
        assert_eq!(parsed["status"], "success");
    }

    // TS-03-7 via process_command: Lock rejected when vehicle moving
    #[tokio::test]
    async fn test_process_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(50.0))
            .with_door_open(Some(false));
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("response should be valid JSON");
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
    }

    // TS-03-8 via process_command: Lock rejected when door open
    #[tokio::test]
    async fn test_process_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(true));
        let mut lock_state = false;
        let response = process_command(&mock, &make_lock_cmd(), &mut lock_state).await;
        let parsed: serde_json::Value =
            serde_json::from_str(&response).expect("response should be valid JSON");
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "door_open");
    }
}
