//! Safety constraint validation for lock commands.
//!
//! Before executing a lock command the service must confirm the vehicle is
//! stationary (speed < 1.0 km/h) and the door is closed. Speed is evaluated
//! first; if speed fails the door state is irrelevant (Property 2).

#![allow(dead_code)]

use crate::broker::{BrokerClient, SIGNAL_IS_OPEN, SIGNAL_SPEED};

// ── SafetyResult ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SafetyResult {
    Safe,
    VehicleMoving,
    DoorOpen,
}

// ── check_safety ──────────────────────────────────────────────────────────────

/// Read safety signals from the broker and determine whether a lock is allowed.
///
/// Speed is evaluated first (03-REQ-3.1 takes priority over 03-REQ-3.2).
/// An unset speed is treated as 0.0 (03-REQ-3.E1).
/// An unset door signal is treated as closed (03-REQ-3.E2).
pub async fn check_safety<B: BrokerClient>(broker: &B) -> SafetyResult {
    // Speed is evaluated first (Property 2: speed check takes priority over door).
    let speed = broker
        .get_float(SIGNAL_SPEED)
        .await
        .unwrap_or(None)
        .unwrap_or(0.0);
    if speed >= 1.0 {
        return SafetyResult::VehicleMoving;
    }

    // Unset door signal is treated as closed (03-REQ-3.E2).
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

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockBrokerClient;

    /// TS-03-7 / 03-REQ-3.1: speed >= 1.0 km/h returns VehicleMoving.
    #[tokio::test]
    async fn test_lock_rejected_vehicle_moving() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(50.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    /// TS-03-8 / 03-REQ-3.2: door ajar at speed 0.0 returns DoorOpen.
    #[tokio::test]
    async fn test_lock_rejected_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(true));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::DoorOpen);
    }

    /// TS-03-9 / 03-REQ-3.3: speed < 1.0 and door closed returns Safe.
    #[tokio::test]
    async fn test_lock_allowed_safe() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    /// Speed exactly at the 1.0 km/h threshold is rejected (>= means 1.0 fails).
    #[tokio::test]
    async fn test_lock_rejected_at_threshold() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(1.0))
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::VehicleMoving);
    }

    /// Speed and door both violate: VehicleMoving is returned (speed checked first).
    #[tokio::test]
    async fn test_speed_priority_over_door_open() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(50.0))
            .with_door_open(Some(true));
        let result = check_safety(&mock).await;
        assert_eq!(
            result,
            SafetyResult::VehicleMoving,
            "speed must be evaluated before door state (Property 2)"
        );
    }

    /// TS-03-E6 / 03-REQ-3.E1: unset speed signal treated as 0.0 (safe default).
    #[tokio::test]
    async fn test_speed_unset_treated_zero() {
        let mock = MockBrokerClient::new()
            .with_speed(None)
            .with_door_open(Some(false));
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }

    /// TS-03-E7 / 03-REQ-3.E2: unset door signal treated as closed (safe default).
    #[tokio::test]
    async fn test_door_unset_treated_closed() {
        let mock = MockBrokerClient::new()
            .with_speed(Some(0.0))
            .with_door_open(None);
        let result = check_safety(&mock).await;
        assert_eq!(result, SafetyResult::Safe);
    }
}
