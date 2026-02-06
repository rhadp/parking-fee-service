//! Property-based tests for safety validation.
//!
//! These tests verify Properties 2 and 3 from the design document:
//! - Property 2: Lock Fails When Door Is Open
//! - Property 3: Unlock Fails When Vehicle Is Moving

use std::time::Duration;

use locking_service::error::{LockingError, SafetyViolation};
use locking_service::proto::Door;
use locking_service::state::LockState;
use locking_service::validator::test_utils::MockSignalReader;
use locking_service::validator::{vss_paths, SafetyValidator};
use proptest::prelude::*;

/// Strategy to generate valid doors (excluding Unknown and All).
fn valid_door_strategy() -> impl Strategy<Value = Door> {
    prop_oneof![
        Just(Door::Driver),
        Just(Door::Passenger),
        Just(Door::RearLeft),
        Just(Door::RearRight),
    ]
}

/// Strategy to generate positive speeds (vehicle is moving).
fn positive_speed_strategy() -> impl Strategy<Value = f32> {
    0.1f32..300.0f32
}

/// Creates a test validator with mock signal reader.
fn create_test_validator() -> (SafetyValidator<MockSignalReader>, MockSignalReader) {
    let reader = MockSignalReader::new();
    let validator = SafetyValidator::new(reader.clone(), Duration::from_millis(100));
    (validator, reader)
}

/// Sets up door signals for a specific door.
fn setup_door_signal(reader: &MockSignalReader, door: Door, is_open: bool) {
    if let Some(path) = vss_paths::door_to_path(door) {
        let signal_path = vss_paths::door_is_open(path);
        reader.set_bool(&signal_path, is_open);
    }
}

/// Helper to run async tests within proptest.
fn run_async<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::runtime::Runtime::new().unwrap().block_on(f)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: locking-service, Property 2: Lock Fails When Door Is Open
    ///
    /// For any Lock command when the door is in the open state (IsOpen = true),
    /// the LOCKING_SERVICE SHALL reject the command with a safety violation error,
    /// and the lock state SHALL remain unchanged.
    ///
    /// Validates: Requirements 1.3
    #[test]
    fn property_lock_fails_when_door_open(door in valid_door_strategy()) {
        let (validator, reader) = create_test_validator();

        // Set door as open
        setup_door_signal(&reader, door, true);

        // Capture initial state
        let initial_state = LockState::default();

        // Attempt to lock
        let result: Result<(), LockingError> = run_async(validator.validate_lock(door));

        // Must be rejected with SafetyError::DoorAjar
        prop_assert!(result.is_err(), "Lock should fail when door {:?} is open", door);

        match result {
            Err(LockingError::SafetyError(SafetyViolation::DoorAjar { door: err_door })) => {
                prop_assert_eq!(err_door, door, "Error should reference the correct door");
            }
            Err(other) => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    format!("Expected SafetyError::DoorAjar, got {:?}", other)
                ));
            }
            Ok(()) => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    format!("Lock should have failed for open door {:?}", door)
                ));
            }
        }

        // Verify gRPC status is FAILED_PRECONDITION
        let status: tonic::Status = LockingError::SafetyError(SafetyViolation::DoorAjar { door }).into();
        prop_assert_eq!(
            status.code(),
            tonic::Code::FailedPrecondition,
            "Open door should result in FAILED_PRECONDITION status"
        );

        // State should remain unchanged (lock state was not modified)
        let final_state = LockState::default();
        prop_assert_eq!(
            initial_state.driver.is_locked,
            final_state.driver.is_locked,
            "Lock state should remain unchanged after rejection"
        );
    }

    /// Feature: locking-service, Property 2: Lock Succeeds When Door Is Closed
    ///
    /// Complementary test: Lock validation should succeed when door is closed.
    #[test]
    fn property_lock_succeeds_when_door_closed(door in valid_door_strategy()) {
        let (validator, reader) = create_test_validator();

        // Set door as closed
        setup_door_signal(&reader, door, false);

        // Attempt to lock
        let result: Result<(), LockingError> = run_async(validator.validate_lock(door));

        // Should succeed
        prop_assert!(result.is_ok(), "Lock should succeed when door {:?} is closed", door);
    }

    /// Feature: locking-service, Property 3: Unlock Fails When Vehicle Is Moving
    ///
    /// For any Unlock command when the vehicle speed is greater than 0,
    /// the LOCKING_SERVICE SHALL reject the command with a safety violation error,
    /// and the lock state SHALL remain unchanged.
    ///
    /// Validates: Requirements 2.3
    #[test]
    fn property_unlock_fails_when_vehicle_moving(speed_kmh in positive_speed_strategy()) {
        let (validator, reader) = create_test_validator();

        // Set vehicle as moving
        reader.set_float(vss_paths::VEHICLE_SPEED, speed_kmh);

        // Capture initial state
        let initial_state = LockState::default();

        // Attempt to unlock
        let result: Result<(), LockingError> = run_async(validator.validate_unlock());

        // Must be rejected with SafetyError::VehicleMoving
        prop_assert!(result.is_err(), "Unlock should fail when vehicle is moving at {} km/h", speed_kmh);

        match result {
            Err(LockingError::SafetyError(SafetyViolation::VehicleMoving { speed_kmh: err_speed })) => {
                prop_assert!(
                    (err_speed - speed_kmh).abs() < f32::EPSILON,
                    "Error should contain correct speed: expected {}, got {}", speed_kmh, err_speed
                );
            }
            Err(other) => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    format!("Expected SafetyError::VehicleMoving, got {:?}", other)
                ));
            }
            Ok(()) => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    "Unlock should have failed for moving vehicle"
                ));
            }
        }

        // Verify gRPC status is FAILED_PRECONDITION
        let status: tonic::Status = LockingError::SafetyError(SafetyViolation::VehicleMoving { speed_kmh }).into();
        prop_assert_eq!(
            status.code(),
            tonic::Code::FailedPrecondition,
            "Moving vehicle should result in FAILED_PRECONDITION status"
        );

        // State should remain unchanged
        let final_state = LockState::default();
        prop_assert_eq!(
            initial_state.driver.is_locked,
            final_state.driver.is_locked,
            "Lock state should remain unchanged after rejection"
        );
    }

    /// Feature: locking-service, Property 3: Unlock Succeeds When Vehicle Is Stationary
    ///
    /// Complementary test: Unlock validation should succeed when vehicle is stationary.
    #[test]
    fn property_unlock_succeeds_when_stationary(_ in Just(0)) {
        let (validator, reader) = create_test_validator();

        // Set vehicle as stationary
        reader.set_float(vss_paths::VEHICLE_SPEED, 0.0);

        // Attempt to unlock
        let result: Result<(), LockingError> = run_async(validator.validate_unlock());

        // Should succeed
        prop_assert!(result.is_ok(), "Unlock should succeed when vehicle is stationary");
    }
}

/// Tests for DATA_BROKER unavailability handling.
mod data_broker_unavailable {
    use super::*;

    #[tokio::test]
    async fn lock_fails_when_data_broker_unavailable() {
        let (validator, reader) = create_test_validator();

        // Don't set any signals - simulates DATA_BROKER unavailable
        reader.set_bool_error(
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
            "DATA_BROKER unavailable",
        );

        let result: Result<(), LockingError> = validator.validate_lock(Door::Driver).await;

        assert!(result.is_err());
        match &result {
            Err(LockingError::DataBrokerError(msg)) => {
                assert!(msg.contains("unavailable"));
            }
            _ => panic!("Expected DataBrokerError"),
        }

        // Verify gRPC status is UNAVAILABLE
        let status: tonic::Status = result.unwrap_err().into();
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }

    #[tokio::test]
    async fn unlock_fails_when_data_broker_unavailable() {
        let (validator, reader) = create_test_validator();

        // Simulate DATA_BROKER unavailable
        reader.set_float_error(vss_paths::VEHICLE_SPEED, "DATA_BROKER unavailable");

        let result: Result<(), LockingError> = validator.validate_unlock().await;

        assert!(result.is_err());
        match &result {
            Err(LockingError::DataBrokerError(msg)) => {
                assert!(msg.contains("unavailable"));
            }
            _ => panic!("Expected DataBrokerError"),
        }

        // Verify gRPC status is UNAVAILABLE
        let status: tonic::Status = result.unwrap_err().into();
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }
}
