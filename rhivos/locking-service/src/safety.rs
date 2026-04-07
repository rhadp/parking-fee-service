use crate::broker::BrokerClient;

/// Result of safety constraint validation.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyResult {
    /// All safety constraints met.
    Safe,
    /// Vehicle speed >= 1.0 km/h.
    VehicleMoving,
    /// Door is ajar.
    DoorOpen,
}

/// Check safety constraints by reading Vehicle.Speed and door open status.
pub async fn check_safety<B: BrokerClient>(_broker: &B) -> SafetyResult {
    todo!("check_safety not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockBrokerClient;

    /// TS-03-7: Verify lock rejected with VehicleMoving when speed >= 1.0.
    #[tokio::test]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(50.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    /// TS-03-8: Verify lock rejected with DoorOpen when door is ajar.
    #[tokio::test]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(true));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::DoorOpen);
    }

    /// TS-03-9: Verify lock allowed when speed < 1.0 and door closed.
    #[tokio::test]
    async fn test_lock_allowed_safe() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    /// TS-03-E6: Verify unset speed signal treated as 0.0 (safe default).
    #[tokio::test]
    async fn test_speed_unset_treated_zero() {
        let mock = MockBrokerClient::new()
            .with_speed(None)
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    /// TS-03-E7: Verify unset door signal treated as closed (safe default).
    #[tokio::test]
    async fn test_door_unset_treated_closed() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(None);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }
}
