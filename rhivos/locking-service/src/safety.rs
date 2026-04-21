//! Safety constraint validation for lock commands.
use crate::broker::BrokerClient;

/// Result of a safety constraint check.
#[derive(Debug, PartialEq)]
pub enum SafetyResult {
    Safe,
    VehicleMoving,
    DoorOpen,
}

/// Check safety constraints before executing a lock command.
///
/// Reads Vehicle.Speed and Vehicle.Cabin.Door.Row1.DriverSide.IsOpen.
/// Speed is checked first (ASIL-B priority ordering).
/// None speed → treated as 0.0; None door → treated as false (safe defaults).
pub async fn check_safety<B: BrokerClient>(_broker: &B) -> SafetyResult {
    todo!("implemented in task group 2")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockBrokerClient;

    // TS-03-7: Speed >= 1.0 returns VehicleMoving
    #[tokio::test]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(50.0)
            .with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    // TS-03-8: Door ajar returns DoorOpen (speed = 0.0)
    #[tokio::test]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(true);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::DoorOpen);
    }

    // TS-03-9: Speed < 1.0 and door closed returns Safe
    #[tokio::test]
    async fn test_lock_allowed_safe() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E6: None speed treated as 0.0 (safe default)
    #[tokio::test]
    async fn test_speed_unset_treated_zero() {
        let mock = MockBrokerClient::new()
            .with_speed_none()
            .with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E7: None door treated as false (safe default)
    #[tokio::test]
    async fn test_door_unset_treated_closed() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .with_door_open_none();
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // Boundary: speed exactly 1.0 should return VehicleMoving (>= 1.0 threshold)
    #[tokio::test]
    async fn test_speed_at_threshold_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(1.0)
            .with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    // Boundary: speed just below 1.0 should return Safe
    #[tokio::test]
    async fn test_speed_just_below_threshold_safe() {
        let mock = MockBrokerClient::new()
            .with_speed(0.99)
            .with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // Both speed >= 1.0 and door open: speed is checked first → VehicleMoving
    #[tokio::test]
    async fn test_speed_takes_priority_over_door() {
        let mock = MockBrokerClient::new()
            .with_speed(50.0)
            .with_door_open(true);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving, "speed check must take priority");
    }
}
