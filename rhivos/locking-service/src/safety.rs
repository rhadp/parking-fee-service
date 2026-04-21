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
pub async fn check_safety<B: BrokerClient>(broker: &B) -> SafetyResult {
    use crate::broker::{SIGNAL_IS_OPEN, SIGNAL_SPEED};

    // Read speed; None or error → treat as 0.0 (safe default per 03-REQ-3.E1)
    let speed = broker
        .get_float(SIGNAL_SPEED)
        .await
        .unwrap_or(None)
        .unwrap_or(0.0);

    // Speed check takes priority (03-REQ-3.1, Property 2)
    if speed >= 1.0 {
        return SafetyResult::VehicleMoving;
    }

    // Read door state; None or error → treat as false (safe default per 03-REQ-3.E2)
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

    // Addresses major review finding [03-REQ-5.2]: broker error during get_float
    // (speed check) is treated as safe default (speed = 0.0).
    // The spec does not define behavior for broker errors during safety checks;
    // the implementation uses unwrap_or(None) → 0.0 (safe default).
    #[tokio::test]
    async fn test_get_float_error_treated_as_safe() {
        let mock = MockBrokerClient::new()
            .fail_get_float()
            .with_door_open(false);
        let result = check_safety(&mock).await;
        assert_eq!(
            result,
            SafetyResult::Safe,
            "broker error on speed read must be treated as 0.0 (safe default)"
        );
    }

    // Addresses major review finding [03-REQ-5.2]: broker error during get_bool
    // (door check) is treated as safe default (door closed).
    #[tokio::test]
    async fn test_get_bool_error_treated_as_safe() {
        let mock = MockBrokerClient::new()
            .with_speed(0.0)
            .fail_get_bool();
        let result = check_safety(&mock).await;
        assert_eq!(
            result,
            SafetyResult::Safe,
            "broker error on door read must be treated as closed (safe default)"
        );
    }

    // Both broker calls fail: both treated as safe defaults → Safe
    #[tokio::test]
    async fn test_both_broker_errors_treated_as_safe() {
        let mock = MockBrokerClient::new()
            .fail_get_float()
            .fail_get_bool();
        let result = check_safety(&mock).await;
        assert_eq!(
            result,
            SafetyResult::Safe,
            "all broker errors must be treated as safe defaults"
        );
    }
}
