//! Safety constraint checking for the LOCKING_SERVICE.
//!
//! Validates that lock/unlock commands can be safely executed by reading
//! vehicle state signals from DATA_BROKER. Constraints:
//!
//! - Vehicle must be stationary (Vehicle.Speed == 0 or not set)
//! - Door must be closed for lock commands (IsOpen == false or not set)

use databroker_client::{DataValue, DatabrokerClient};
use tracing::{debug, warn};

use crate::command::reason;

/// VSS path for vehicle speed.
const SPEED_PATH: &str = "Vehicle.Speed";

/// VSS path for door open state.
const DOOR_OPEN_PATH: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

/// Checks safety constraints against the current vehicle state from DATA_BROKER.
pub struct SafetyChecker {
    db_client: DatabrokerClient,
}

impl SafetyChecker {
    /// Create a new safety checker using the given DATA_BROKER client.
    pub fn new(db_client: DatabrokerClient) -> Self {
        Self { db_client }
    }

    /// Check all safety constraints for a lock command.
    ///
    /// Checks:
    /// 1. Vehicle speed must be 0 (or not set, which defaults to safe per 02-REQ-3.E1)
    /// 2. Door must be closed (or not set, which defaults to safe per 02-REQ-3.E2)
    ///
    /// Returns `Ok(())` if all constraints pass, or `Err(reason)` with the
    /// specific constraint violated.
    pub async fn check_lock_constraints(&self) -> Result<(), String> {
        self.check_speed().await?;
        self.check_door().await?;
        Ok(())
    }

    /// Check all safety constraints for an unlock command.
    ///
    /// Checks:
    /// 1. Vehicle speed must be 0 (or not set)
    ///
    /// Note: door ajar check is NOT required for unlock (only for lock).
    ///
    /// Returns `Ok(())` if all constraints pass, or `Err(reason)` with the
    /// specific constraint violated.
    pub async fn check_unlock_constraints(&self) -> Result<(), String> {
        self.check_speed().await?;
        Ok(())
    }

    /// Check the vehicle speed constraint.
    ///
    /// Reads Vehicle.Speed from DATA_BROKER. If the value is greater than zero,
    /// returns Err("vehicle_moving"). If the signal has not been set (NoValue),
    /// treats speed as 0 (safe) per 02-REQ-3.E1.
    async fn check_speed(&self) -> Result<(), String> {
        match self.db_client.get_value_opt(SPEED_PATH).await {
            Ok(Some(DataValue::Float(speed))) => {
                debug!(speed = speed, "read vehicle speed");
                if speed > 0.0 {
                    return Err(reason::VEHICLE_MOVING.to_string());
                }
                Ok(())
            }
            Ok(Some(DataValue::Double(speed))) => {
                debug!(speed = speed, "read vehicle speed (double)");
                if speed > 0.0 {
                    return Err(reason::VEHICLE_MOVING.to_string());
                }
                Ok(())
            }
            Ok(None) => {
                debug!("vehicle speed not set, treating as 0 (safe)");
                Ok(())
            }
            Ok(Some(other)) => {
                warn!(value = ?other, "unexpected type for Vehicle.Speed, treating as safe");
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "failed to read Vehicle.Speed, treating as safe");
                Ok(())
            }
        }
    }

    /// Check the door ajar constraint.
    ///
    /// Reads Vehicle.Cabin.Door.Row1.DriverSide.IsOpen from DATA_BROKER.
    /// If the value is true (door open), returns Err("door_open").
    /// If the signal has not been set (NoValue), treats door as closed (safe)
    /// per 02-REQ-3.E2.
    async fn check_door(&self) -> Result<(), String> {
        match self.db_client.get_value_opt(DOOR_OPEN_PATH).await {
            Ok(Some(DataValue::Bool(is_open))) => {
                debug!(is_open = is_open, "read door open state");
                if is_open {
                    return Err(reason::DOOR_OPEN.to_string());
                }
                Ok(())
            }
            Ok(None) => {
                debug!("door open state not set, treating as closed (safe)");
                Ok(())
            }
            Ok(Some(other)) => {
                warn!(value = ?other, "unexpected type for IsOpen, treating as safe");
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "failed to read IsOpen, treating as safe");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Safety checker tests require a running DATA_BROKER (integration tests).
    // Unit-level logic is tested via command.rs reason constants.
    // See safety-tests/tests/locking_service_tests.rs for integration tests.
}
