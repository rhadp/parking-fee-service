use crate::broker::BrokerClient;
use crate::command::LockCommand;

/// Process a validated lock/unlock command.
///
/// For lock: checks safety constraints, updates state if changed.
/// For unlock: skips safety, updates state if changed.
/// Returns the JSON response string.
pub async fn process_command<B: BrokerClient>(
    _broker: &B,
    _cmd: &LockCommand,
    _lock_state: &mut bool,
) -> String {
    todo!()
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
