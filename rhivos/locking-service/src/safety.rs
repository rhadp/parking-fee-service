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

/// Signal path for vehicle speed.
const SIGNAL_SPEED: &str = "Vehicle.Speed";
/// Signal path for door open status.
const SIGNAL_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

/// Check safety constraints by reading Vehicle.Speed and door open status.
///
/// Speed is checked first; if speed >= 1.0 the result is VehicleMoving
/// regardless of door state. If speed signal is unset, it is treated as 0.0.
/// If door signal is unset, it is treated as closed (false).
pub async fn check_safety<B: BrokerClient>(broker: &B) -> SafetyResult {
    // Read speed — treat unset as 0.0 (safe default per 03-REQ-3.E1)
    let speed = broker
        .get_float(SIGNAL_SPEED)
        .await
        .unwrap_or(None)
        .unwrap_or(0.0);

    if speed >= 1.0 {
        return SafetyResult::VehicleMoving;
    }

    // Read door open — treat unset as false (safe default per 03-REQ-3.E2)
    let door_open = broker
        .get_bool(SIGNAL_IS_OPEN)
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
