//! Safety constraint validation for lock commands.
//!
//! `check_safety` reads `Vehicle.Speed` and `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`
//! from DATA_BROKER and returns a `SafetyResult`.  Speed is checked first
//! (design.md Property 2).

use crate::broker::{BrokerClient, SIGNAL_IS_OPEN, SIGNAL_SPEED};

/// Outcome of the safety constraint check.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyResult {
    /// All constraints satisfied; lock command may proceed.
    Safe,
    /// Vehicle speed ≥ 1.0 km/h; lock command must be rejected.
    VehicleMoving,
    /// Door is ajar (IsOpen == true); lock command must be rejected.
    DoorOpen,
}

/// Read safety signals from `broker` and return the constraint status.
///
/// Speed is checked before door state (Property 2 of design.md).
/// - Speed signal absent → treated as 0.0 (safe default, 03-REQ-3.E1)
/// - Door signal absent  → treated as false (closed, safe default, 03-REQ-3.E2)
pub async fn check_safety<B: BrokerClient>(broker: &B) -> SafetyResult {
    // Check speed first — it takes priority over door state (03-REQ-3.1, design.md Property 2).
    let speed = broker
        .get_float(SIGNAL_SPEED)
        .await
        .unwrap_or(None)
        .unwrap_or(0.0);

    if speed >= 1.0 {
        return SafetyResult::VehicleMoving;
    }

    // Check door state (03-REQ-3.2).
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

    // TS-03-7: Speed >= 1.0 returns VehicleMoving
    #[tokio::test(flavor = "current_thread")]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new().with_speed(50.0).with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    // TS-03-8: Door open returns DoorOpen (speed = 0.0)
    #[tokio::test(flavor = "current_thread")]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(true);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::DoorOpen);
    }

    // TS-03-9: Speed < 1.0 and door closed returns Safe
    #[tokio::test(flavor = "current_thread")]
    async fn test_lock_allowed_safe() {
        let mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E6: Speed signal unset → treated as 0.0 (safe default)
    #[tokio::test(flavor = "current_thread")]
    async fn test_speed_unset_treated_zero() {
        // speed defaults to None in MockBrokerClient::new()
        let mock = MockBrokerClient::new().with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // TS-03-E7: Door signal unset → treated as closed (safe default)
    #[tokio::test(flavor = "current_thread")]
    async fn test_door_unset_treated_closed() {
        // door_open defaults to None in MockBrokerClient::new()
        let mock = MockBrokerClient::new().with_speed(0.0);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    // Additional: speed exactly at boundary (0.9 = safe, 1.0 = moving)
    #[tokio::test(flavor = "current_thread")]
    async fn test_speed_boundary_safe() {
        let mock = MockBrokerClient::new().with_speed(0.9).with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_speed_boundary_moving() {
        let mock = MockBrokerClient::new().with_speed(1.0).with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    // Speed >= 1.0 takes priority over door open (design.md Property 2)
    #[tokio::test(flavor = "current_thread")]
    async fn test_speed_priority_over_door() {
        let mock = MockBrokerClient::new().with_speed(50.0).with_door_open(true);
        let result = check_safety(&mock).await;
        assert_eq!(
            result,
            SafetyResult::VehicleMoving,
            "speed check must take priority over door state"
        );
    }
}
