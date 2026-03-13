use crate::broker::BrokerClient;

/// VSS signal paths
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

/// Result of safety constraint evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyResult {
    Safe,
    VehicleMoving,
    DoorOpen,
}

/// Check safety constraints for a lock operation by reading signals from the broker.
/// Speed signal: None → treated as 0.0 (safe). Any positive value → VehicleMoving.
/// Door signal: None → treated as false (closed). true → DoorOpen.
pub async fn check_safety<B: BrokerClient>(broker: &B) -> SafetyResult {
    let speed = broker
        .get_float(SIGNAL_SPEED)
        .await
        .unwrap_or(None)
        .unwrap_or(0.0);
    if speed > 0.0 {
        return SafetyResult::VehicleMoving;
    }
    let door_open = broker
        .get_bool(SIGNAL_DOOR_OPEN)
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

    // TS-03-7: Lock Rejected When Vehicle Moving
    #[tokio::test]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(50.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    // TS-03-8: Lock Rejected When Door Open
    #[tokio::test]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(true));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::DoorOpen);
    }

    // TS-03-9: Lock Allowed When Safe
    #[tokio::test]
    async fn test_lock_allowed_safe() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E6: Speed Signal Unset
    #[tokio::test]
    async fn test_speed_unset_treated_zero() {
        let mock = MockBrokerClient::new()
            .with_speed(None)
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E7: Door Signal Unset
    #[tokio::test]
    async fn test_door_unset_treated_closed() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(None);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }
}
