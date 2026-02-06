//! Safety constraint validation for LOCKING_SERVICE.
//!
//! This module provides safety validation before executing lock/unlock commands.
//! It reads vehicle signals from the DATA_BROKER to ensure operations are safe.

use std::time::Duration;

use crate::error::{LockingError, SafetyViolation};
use crate::proto::Door;

/// VSS signal paths for safety validation.
pub mod vss_paths {
    use crate::proto::Door;

    /// Base path for door signals.
    pub const DOOR_BASE: &str = "Vehicle.Cabin.Door";
    /// Vehicle speed signal path.
    pub const VEHICLE_SPEED: &str = "Vehicle.Speed";

    /// Returns the IsOpen signal path for a specific door.
    pub fn door_is_open(door: &str) -> String {
        format!("{}.{}.IsOpen", DOOR_BASE, door)
    }

    /// Returns the IsLocked signal path for a specific door.
    pub fn door_is_locked(door: &str) -> String {
        format!("{}.{}.IsLocked", DOOR_BASE, door)
    }

    /// Maps a Door enum to its VSS path component.
    pub fn door_to_path(door: Door) -> Option<&'static str> {
        match door {
            Door::Driver => Some("Row1.DriverSide"),
            Door::Passenger => Some("Row1.PassengerSide"),
            Door::RearLeft => Some("Row2.DriverSide"),
            Door::RearRight => Some("Row2.PassengerSide"),
            Door::Unknown | Door::All => None,
        }
    }
}

/// Trait for reading vehicle signals from DATA_BROKER.
///
/// This trait abstracts the data broker client to enable testing with mocks.
#[async_trait::async_trait]
pub trait SignalReader: Send + Sync {
    /// Reads a boolean signal value.
    async fn read_bool(&self, path: &str) -> Result<bool, LockingError>;

    /// Reads a float signal value.
    async fn read_float(&self, path: &str) -> Result<f32, LockingError>;
}

/// Safety validator that checks constraints before lock/unlock operations.
pub struct SafetyValidator<R: SignalReader> {
    signal_reader: R,
    validation_timeout: Duration,
}

impl<R: SignalReader> SafetyValidator<R> {
    /// Creates a new SafetyValidator with the given signal reader and timeout.
    pub fn new(signal_reader: R, validation_timeout: Duration) -> Self {
        Self {
            signal_reader,
            validation_timeout,
        }
    }

    /// Returns the configured validation timeout.
    pub fn validation_timeout(&self) -> Duration {
        self.validation_timeout
    }

    /// Validates safety constraints for a lock operation.
    ///
    /// A lock operation is rejected if:
    /// - The door is physically open (IsOpen = true)
    /// - The DATA_BROKER is unavailable
    ///
    /// # Arguments
    ///
    /// * `door` - The door to validate for locking
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the door can be safely locked
    /// * `Err(LockingError::SafetyError)` if the door is open
    /// * `Err(LockingError::DataBrokerError)` if signals cannot be read
    /// * `Err(LockingError::InvalidDoor)` if the door is invalid
    pub async fn validate_lock(&self, door: Door) -> Result<(), LockingError> {
        // Validate door is valid
        let door_path = vss_paths::door_to_path(door).ok_or(LockingError::InvalidDoor(door))?;

        // Check if door is open
        let signal_path = vss_paths::door_is_open(door_path);

        let is_open = tokio::time::timeout(
            self.validation_timeout,
            self.signal_reader.read_bool(&signal_path),
        )
        .await
        .map_err(|_| LockingError::TimeoutError(self.validation_timeout.as_millis() as u64))??;

        if is_open {
            return Err(LockingError::SafetyError(SafetyViolation::DoorAjar {
                door,
            }));
        }

        Ok(())
    }

    /// Validates safety constraints for an unlock operation.
    ///
    /// An unlock operation is rejected if:
    /// - The vehicle is moving (Speed > 0)
    /// - The DATA_BROKER is unavailable
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the doors can be safely unlocked
    /// * `Err(LockingError::SafetyError)` if the vehicle is moving
    /// * `Err(LockingError::DataBrokerError)` if signals cannot be read
    pub async fn validate_unlock(&self) -> Result<(), LockingError> {
        let speed_kmh = tokio::time::timeout(
            self.validation_timeout,
            self.signal_reader.read_float(vss_paths::VEHICLE_SPEED),
        )
        .await
        .map_err(|_| LockingError::TimeoutError(self.validation_timeout.as_millis() as u64))??;

        if speed_kmh > 0.0 {
            return Err(LockingError::SafetyError(SafetyViolation::VehicleMoving {
                speed_kmh,
            }));
        }

        Ok(())
    }
}

/// Test utilities for mocking the signal reader.
/// Made available for integration tests.
pub mod test_utils {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    /// Mock signal reader for testing.
    #[derive(Clone, Default)]
    pub struct MockSignalReader {
        bool_signals: Arc<RwLock<HashMap<String, Result<bool, String>>>>,
        float_signals: Arc<RwLock<HashMap<String, Result<f32, String>>>>,
    }

    impl MockSignalReader {
        /// Creates a new mock signal reader.
        pub fn new() -> Self {
            Self::default()
        }

        /// Sets a boolean signal value.
        pub fn set_bool(&self, path: &str, value: bool) {
            self.bool_signals
                .write()
                .unwrap()
                .insert(path.to_string(), Ok(value));
        }

        /// Sets a boolean signal to return an error.
        pub fn set_bool_error(&self, path: &str, error: &str) {
            self.bool_signals
                .write()
                .unwrap()
                .insert(path.to_string(), Err(error.to_string()));
        }

        /// Sets a float signal value.
        pub fn set_float(&self, path: &str, value: f32) {
            self.float_signals
                .write()
                .unwrap()
                .insert(path.to_string(), Ok(value));
        }

        /// Sets a float signal to return an error.
        pub fn set_float_error(&self, path: &str, error: &str) {
            self.float_signals
                .write()
                .unwrap()
                .insert(path.to_string(), Err(error.to_string()));
        }
    }

    #[async_trait::async_trait]
    impl SignalReader for MockSignalReader {
        async fn read_bool(&self, path: &str) -> Result<bool, LockingError> {
            self.bool_signals
                .read()
                .unwrap()
                .get(path)
                .cloned()
                .unwrap_or(Err("signal not found".to_string()))
                .map_err(LockingError::DataBrokerError)
        }

        async fn read_float(&self, path: &str) -> Result<f32, LockingError> {
            self.float_signals
                .read()
                .unwrap()
                .get(path)
                .cloned()
                .unwrap_or(Err("signal not found".to_string()))
                .map_err(LockingError::DataBrokerError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils::MockSignalReader;

    fn create_test_validator() -> (SafetyValidator<MockSignalReader>, MockSignalReader) {
        let reader = MockSignalReader::new();
        let validator = SafetyValidator::new(reader.clone(), Duration::from_millis(100));
        (validator, reader)
    }

    #[tokio::test]
    async fn test_validate_lock_door_closed() {
        let (validator, reader) = create_test_validator();
        reader.set_bool("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", false);

        let result = validator.validate_lock(Door::Driver).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_lock_door_open() {
        let (validator, reader) = create_test_validator();
        reader.set_bool("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", true);

        let result = validator.validate_lock(Door::Driver).await;
        assert!(result.is_err());
        match result {
            Err(LockingError::SafetyError(SafetyViolation::DoorAjar { door })) => {
                assert_eq!(door, Door::Driver);
            }
            _ => panic!("Expected SafetyError::DoorAjar"),
        }
    }

    #[tokio::test]
    async fn test_validate_lock_invalid_door() {
        let (validator, _reader) = create_test_validator();

        let result = validator.validate_lock(Door::Unknown).await;
        assert!(matches!(result, Err(LockingError::InvalidDoor(_))));
    }

    #[tokio::test]
    async fn test_validate_lock_data_broker_error() {
        let (validator, reader) = create_test_validator();
        reader.set_bool_error(
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
            "connection failed",
        );

        let result = validator.validate_lock(Door::Driver).await;
        assert!(matches!(result, Err(LockingError::DataBrokerError(_))));
    }

    #[tokio::test]
    async fn test_validate_unlock_stationary() {
        let (validator, reader) = create_test_validator();
        reader.set_float("Vehicle.Speed", 0.0);

        let result = validator.validate_unlock().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_unlock_moving() {
        let (validator, reader) = create_test_validator();
        reader.set_float("Vehicle.Speed", 30.0);

        let result = validator.validate_unlock().await;
        assert!(result.is_err());
        match result {
            Err(LockingError::SafetyError(SafetyViolation::VehicleMoving { speed_kmh })) => {
                assert!((speed_kmh - 30.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected SafetyError::VehicleMoving"),
        }
    }

    #[tokio::test]
    async fn test_validate_unlock_data_broker_error() {
        let (validator, reader) = create_test_validator();
        reader.set_float_error("Vehicle.Speed", "connection failed");

        let result = validator.validate_unlock().await;
        assert!(matches!(result, Err(LockingError::DataBrokerError(_))));
    }

    #[test]
    fn test_vss_paths() {
        assert_eq!(
            vss_paths::door_is_open("Row1.DriverSide"),
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
        );
        assert_eq!(
            vss_paths::door_to_path(Door::Driver),
            Some("Row1.DriverSide")
        );
        assert_eq!(
            vss_paths::door_to_path(Door::Passenger),
            Some("Row1.PassengerSide")
        );
        assert_eq!(vss_paths::door_to_path(Door::Unknown), None);
    }
}
