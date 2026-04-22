use crate::broker::BrokerClient;

/// Result of a safety constraint check.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyResult {
    /// All safety constraints are met.
    Safe,
    /// Vehicle speed is >= 1.0 km/h.
    VehicleMoving,
    /// Door is ajar (open).
    DoorOpen,
}

/// Check safety constraints for a lock command.
///
/// Reads Vehicle.Speed and door open state from the broker.
/// Speed is checked first; if >= 1.0 returns VehicleMoving.
/// Then door is checked; if open returns DoorOpen.
/// If both are safe, returns Safe.
///
/// Unset speed is treated as 0.0 (safe default).
/// Unset door state is treated as closed (safe default).
pub async fn check_safety<B: BrokerClient>(broker: &B) -> SafetyResult {
    // Read speed; treat errors or unset values as 0.0 (safe default).
    let speed = broker
        .get_float(crate::broker::SIGNAL_SPEED)
        .await
        .unwrap_or(None)
        .unwrap_or(0.0);

    // Speed check takes priority over door check (design Property 2).
    if speed >= 1.0 {
        return SafetyResult::VehicleMoving;
    }

    // Read door open state; treat errors or unset values as false (safe default).
    let door_open = broker
        .get_bool(crate::broker::SIGNAL_DOOR_OPEN)
        .await
        .unwrap_or(None)
        .unwrap_or(false);

    if door_open {
        return SafetyResult::DoorOpen;
    }

    SafetyResult::Safe
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockBrokerClient;

    // TS-03-7: Verify lock rejected when vehicle moving (speed >= 1.0).
    #[tokio::test]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(50.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    // TS-03-8: Verify lock rejected when door open and vehicle stationary.
    #[tokio::test]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(true));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::DoorOpen);
    }

    // TS-03-9: Verify lock allowed when speed < 1.0 and door closed.
    #[tokio::test]
    async fn test_lock_allowed_safe() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E6: Verify unset speed signal treated as 0.0 (safe).
    #[tokio::test]
    async fn test_speed_unset_treated_zero() {
        let mock = MockBrokerClient::new()
            .with_speed(None)
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E7: Verify unset door signal treated as closed (safe).
    #[tokio::test]
    async fn test_door_unset_treated_closed() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(None);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }
}
