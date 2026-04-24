use crate::broker::{BrokerClient, SIGNAL_IS_OPEN, SIGNAL_SPEED};

#[derive(Debug, PartialEq)]
pub enum SafetyResult {
    Safe,
    VehicleMoving,
    DoorOpen,
}

/// Check safety constraints for a lock command.
///
/// Reads `Vehicle.Speed` and `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from
/// the broker. Speed is checked first (priority rule per design Property 2).
///
/// - Speed >= 1.0 km/h → `VehicleMoving`
/// - Door open (true) → `DoorOpen`
/// - Otherwise → `Safe`
///
/// Missing signals default to safe values: speed = 0.0, door = closed.
pub async fn check_safety<B: BrokerClient>(broker: &B) -> SafetyResult {
    // Read speed, defaulting to 0.0 if signal is unset or read fails
    let speed = broker
        .get_float(SIGNAL_SPEED)
        .await
        .unwrap_or(None)
        .unwrap_or(0.0);

    if speed >= 1.0 {
        return SafetyResult::VehicleMoving;
    }

    // Read door-open state, defaulting to false (closed) if unset or read fails
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

    // TS-03-7: speed >= 1.0 returns VehicleMoving
    #[tokio::test]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(50.0_f32)
            .with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    // TS-03-8: door ajar returns DoorOpen
    #[tokio::test]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0_f32)
            .with_door_open(true);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::DoorOpen);
    }

    // TS-03-9: speed < 1.0 and door closed returns Safe
    #[tokio::test]
    async fn test_lock_allowed_safe() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0_f32)
            .with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E6: None speed treated as 0.0
    #[tokio::test]
    async fn test_speed_unset_treated_zero() {
        let mock = MockBrokerClient::new()
            .with_speed(None::<f32>)
            .with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E7: None door treated as false
    #[tokio::test]
    async fn test_door_unset_treated_closed() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0_f32)
            .with_door_open(None::<bool>);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }
}
