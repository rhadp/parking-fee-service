//! Error types for LOCKING_SERVICE.
//!
//! This module defines the error types used throughout the locking service,
//! including safety violations, authentication errors, and gRPC status mappings.

use crate::proto::Door;

/// Safety constraint violation types.
#[derive(Debug, Clone)]
pub enum SafetyViolation {
    /// Door is physically open and cannot be locked.
    DoorAjar { door: Door },
    /// Vehicle is moving and doors cannot be unlocked.
    VehicleMoving { speed_kmh: f32 },
}

impl std::fmt::Display for SafetyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SafetyViolation::DoorAjar { door } => {
                write!(f, "Door {:?} is open and cannot be locked", door)
            }
            SafetyViolation::VehicleMoving { speed_kmh } => {
                write!(f, "Vehicle is moving at {} km/h, cannot unlock", speed_kmh)
            }
        }
    }
}

/// Main error type for the LOCKING_SERVICE.
#[derive(Debug, thiserror::Error)]
pub enum LockingError {
    /// Authentication failed due to invalid or missing token.
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// Safety constraint was violated.
    #[error("Safety constraint violated: {0}")]
    SafetyError(SafetyViolation),

    /// Lock/unlock execution failed.
    #[error("Execution failed: {0}")]
    ExecutionError(String),

    /// DATA_BROKER is unavailable or connection failed.
    #[error("DATA_BROKER unavailable: {0}")]
    DataBrokerError(String),

    /// Command execution exceeded timeout.
    #[error("Command timeout after {0}ms")]
    TimeoutError(u64),

    /// Invalid door identifier was provided.
    #[error("Invalid door: {0:?}")]
    InvalidDoor(Door),
}

/// Error codes for gRPC error details.
pub mod error_codes {
    /// Authentication token is invalid or missing.
    pub const AUTH_INVALID_TOKEN: &str = "AUTH_INVALID_TOKEN";
    /// Door is physically open (ajar).
    pub const SAFETY_DOOR_AJAR: &str = "SAFETY_DOOR_AJAR";
    /// Vehicle is moving.
    pub const SAFETY_VEHICLE_MOVING: &str = "SAFETY_VEHICLE_MOVING";
    /// Invalid door identifier.
    pub const INVALID_DOOR: &str = "INVALID_DOOR";
    /// DATA_BROKER service is unavailable.
    pub const DATABROKER_UNAVAILABLE: &str = "DATABROKER_UNAVAILABLE";
    /// Command timed out.
    pub const COMMAND_TIMEOUT: &str = "COMMAND_TIMEOUT";
    /// State publication to DATA_BROKER failed (partial success).
    pub const PUBLISH_FAILED: &str = "PUBLISH_FAILED";
}

impl From<LockingError> for tonic::Status {
    fn from(err: LockingError) -> Self {
        match err {
            LockingError::AuthError(msg) => tonic::Status::unauthenticated(msg),
            LockingError::SafetyError(SafetyViolation::DoorAjar { door }) => {
                tonic::Status::failed_precondition(format!("Door {:?} is open", door))
            }
            LockingError::SafetyError(SafetyViolation::VehicleMoving { speed_kmh }) => {
                tonic::Status::failed_precondition(format!("Vehicle moving at {} km/h", speed_kmh))
            }
            LockingError::DataBrokerError(msg) => tonic::Status::unavailable(msg),
            LockingError::TimeoutError(ms) => {
                tonic::Status::deadline_exceeded(format!("Command timed out after {}ms", ms))
            }
            LockingError::InvalidDoor(door) => {
                tonic::Status::invalid_argument(format!("Invalid door: {:?}", door))
            }
            LockingError::ExecutionError(msg) => tonic::Status::internal(msg),
        }
    }
}

/// Result type alias using LockingError.
pub type Result<T> = std::result::Result<T, LockingError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_violation_display() {
        let door_ajar = SafetyViolation::DoorAjar { door: Door::Driver };
        assert!(door_ajar.to_string().contains("Driver"));
        assert!(door_ajar.to_string().contains("open"));

        let moving = SafetyViolation::VehicleMoving { speed_kmh: 30.0 };
        assert!(moving.to_string().contains("30"));
        assert!(moving.to_string().contains("moving"));
    }

    #[test]
    fn test_locking_error_to_status() {
        let auth_err = LockingError::AuthError("invalid token".to_string());
        let status: tonic::Status = auth_err.into();
        assert_eq!(status.code(), tonic::Code::Unauthenticated);

        let safety_err =
            LockingError::SafetyError(SafetyViolation::DoorAjar { door: Door::Driver });
        let status: tonic::Status = safety_err.into();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);

        let moving_err =
            LockingError::SafetyError(SafetyViolation::VehicleMoving { speed_kmh: 50.0 });
        let status: tonic::Status = moving_err.into();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);

        let db_err = LockingError::DataBrokerError("connection failed".to_string());
        let status: tonic::Status = db_err.into();
        assert_eq!(status.code(), tonic::Code::Unavailable);

        let timeout_err = LockingError::TimeoutError(500);
        let status: tonic::Status = timeout_err.into();
        assert_eq!(status.code(), tonic::Code::DeadlineExceeded);

        let invalid_door = LockingError::InvalidDoor(Door::Unknown);
        let status: tonic::Status = invalid_door.into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }
}
