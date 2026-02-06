//! Error types for parking-operator-adaptor.
//!
//! This module defines error types for all parking operations.

use thiserror::Error;
use tonic::Status;

/// Main error type for parking operations.
#[derive(Debug, Error)]
pub enum ParkingError {
    /// Session is already active
    #[error("Session already active: {0}")]
    SessionAlreadyActive(String),

    /// No active session exists
    #[error("No active session")]
    NoActiveSession,

    /// Session operation is in progress
    #[error("Session operation in progress")]
    OperationInProgress,

    /// Location is unavailable
    #[error("Location unavailable: {0}")]
    LocationUnavailable(String),

    /// Parking operator API error
    #[error("Operator API error: {0}")]
    OperatorApiError(String),

    /// DATA_BROKER error
    #[error("DATA_BROKER error: {0}")]
    DataBrokerError(String),

    /// DATA_BROKER connection lost
    #[error("DATA_BROKER connection lost")]
    DataBrokerConnectionLost,

    /// Session storage error
    #[error("Session storage error: {0}")]
    StorageError(String),

    /// API timeout
    #[error("API timeout after {0}ms")]
    ApiTimeout(u64),

    /// Zone lookup failed
    #[error("Zone lookup failed: {0}")]
    ZoneLookupFailed(String),

    /// No parking zone found
    #[error("No parking zone found at location")]
    NoZoneFound,
}

impl From<ParkingError> for Status {
    fn from(err: ParkingError) -> Self {
        match err {
            ParkingError::SessionAlreadyActive(id) => {
                Status::already_exists(format!("Session already active: {}", id))
            }
            ParkingError::NoActiveSession => Status::not_found("No active session"),
            ParkingError::OperationInProgress => {
                Status::failed_precondition("Session operation already in progress")
            }
            ParkingError::LocationUnavailable(msg) => {
                Status::failed_precondition(format!("Location unavailable: {}", msg))
            }
            ParkingError::OperatorApiError(msg) => {
                Status::unavailable(format!("Parking operator API error: {}", msg))
            }
            ParkingError::DataBrokerError(msg) => {
                Status::unavailable(format!("DATA_BROKER error: {}", msg))
            }
            ParkingError::DataBrokerConnectionLost => {
                Status::unavailable("DATA_BROKER connection lost")
            }
            ParkingError::ApiTimeout(ms) => {
                Status::deadline_exceeded(format!("API timeout after {}ms", ms))
            }
            ParkingError::StorageError(msg) => {
                Status::internal(format!("Storage error: {}", msg))
            }
            ParkingError::ZoneLookupFailed(msg) => {
                Status::unavailable(format!("Zone lookup failed: {}", msg))
            }
            ParkingError::NoZoneFound => {
                Status::not_found("No parking zone found at location")
            }
        }
    }
}

/// API error type for HTTP client operations.
#[derive(Debug, Clone, Error)]
pub enum ApiError {
    /// HTTP error with status code
    #[error("HTTP error: {status} - {message}")]
    HttpError { status: u16, message: String },

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Request timeout
    #[error("Timeout after {0}ms")]
    Timeout(u64),

    /// Invalid response from server
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

impl ApiError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            ApiError::NetworkError(_) => true,
            ApiError::Timeout(_) => true,
            ApiError::HttpError { status, .. } => *status >= 500,
            ApiError::InvalidResponse(_) => false,
        }
    }
}

impl From<ApiError> for ParkingError {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::Timeout(ms) => ParkingError::ApiTimeout(ms),
            _ => ParkingError::OperatorApiError(err.to_string()),
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ApiError::Timeout(10000)
        } else if err.is_connect() {
            ApiError::NetworkError(err.to_string())
        } else if let Some(status) = err.status() {
            ApiError::HttpError {
                status: status.as_u16(),
                message: err.to_string(),
            }
        } else {
            ApiError::NetworkError(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parking_error_to_status() {
        let err = ParkingError::SessionAlreadyActive("session-123".to_string());
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::AlreadyExists);

        let err = ParkingError::NoActiveSession;
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);

        let err = ParkingError::OperationInProgress;
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);

        let err = ParkingError::ApiTimeout(10000);
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::DeadlineExceeded);
    }

    #[test]
    fn test_api_error_is_retryable() {
        assert!(ApiError::NetworkError("connection refused".to_string()).is_retryable());
        assert!(ApiError::Timeout(10000).is_retryable());
        assert!(ApiError::HttpError {
            status: 500,
            message: "Internal Server Error".to_string()
        }
        .is_retryable());
        assert!(ApiError::HttpError {
            status: 503,
            message: "Service Unavailable".to_string()
        }
        .is_retryable());
        assert!(!ApiError::HttpError {
            status: 400,
            message: "Bad Request".to_string()
        }
        .is_retryable());
        assert!(!ApiError::InvalidResponse("bad json".to_string()).is_retryable());
    }

    #[test]
    fn test_api_error_to_parking_error() {
        let api_err = ApiError::Timeout(10000);
        let parking_err: ParkingError = api_err.into();
        assert!(matches!(parking_err, ParkingError::ApiTimeout(10000)));

        let api_err = ApiError::NetworkError("connection refused".to_string());
        let parking_err: ParkingError = api_err.into();
        assert!(matches!(parking_err, ParkingError::OperatorApiError(_)));
    }
}
