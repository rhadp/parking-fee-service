use crate::broker::BrokerClient;

/// Result of a safety constraint check for a lock command.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyResult {
    /// All safety constraints are satisfied.
    Safe,
    /// Vehicle speed is >= 1.0 km/h.
    VehicleMoving,
    /// Driver-side door is ajar.
    DoorOpen,
}

/// Check safety constraints for a lock command.
///
/// Reads Vehicle.Speed and Vehicle.Cabin.Door.Row1.DriverSide.IsOpen from
/// the broker. Speed is checked first; if >= 1.0 km/h, returns VehicleMoving
/// regardless of door state. If speed is safe but door is open, returns
/// DoorOpen. Otherwise returns Safe.
///
/// Unset speed is treated as 0.0 (safe). Unset door state is treated as
/// closed (safe).
pub async fn check_safety<B: BrokerClient>(_broker: &B) -> SafetyResult {
    todo!("check_safety not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockBrokerClient;

    // TS-03-7: Verify lock rejected when speed >= 1.0.
    #[tokio::test]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(50.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    // TS-03-8: Verify lock rejected when door is open and vehicle is stationary.
    #[tokio::test]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(true));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::DoorOpen);
    }

    // TS-03-9: Verify lock allowed when safe (speed < 1.0, door closed).
    #[tokio::test]
    async fn test_lock_allowed_safe() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E6: Verify unset speed signal is treated as 0.0 (safe default).
    #[tokio::test]
    async fn test_speed_unset_treated_zero() {
        let mock = MockBrokerClient::new()
            .with_speed(None)
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E7: Verify unset door signal is treated as closed (safe default).
    #[tokio::test]
    async fn test_door_unset_treated_closed() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(None);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }
}
