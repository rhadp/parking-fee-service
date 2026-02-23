//! Error types for the databroker client.

use thiserror::Error;

/// Errors that can occur when interacting with the Kuksa Databroker.
#[derive(Debug, Error)]
pub enum Error {
    /// The gRPC transport layer returned an error (connection refused, timeout, etc.).
    #[error("transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// The gRPC call returned a non-OK status.
    #[error("gRPC status: {0}")]
    Status(#[from] tonic::Status),

    /// The endpoint string could not be parsed.
    #[error("invalid endpoint: {0}")]
    InvalidEndpoint(String),

    /// The requested signal path was not found in the databroker.
    #[error("signal not found: {path}")]
    SignalNotFound {
        /// VSS path that was requested.
        path: String,
    },

    /// The databroker returned an error for a specific entry.
    #[error("databroker entry error for '{path}': code={code}, reason={reason}")]
    EntryError {
        /// VSS path that caused the error.
        path: String,
        /// HTTP-like error code from the databroker.
        code: u32,
        /// Error reason string.
        reason: String,
    },

    /// A value was expected but the datapoint contained no value.
    #[error("no value set for signal: {path}")]
    NoValue {
        /// VSS path that had no value.
        path: String,
    },
}

impl Error {
    /// Returns `true` if this error indicates a permission denied condition.
    pub fn is_permission_denied(&self) -> bool {
        matches!(self, Error::Status(s) if s.code() == tonic::Code::PermissionDenied)
            || matches!(self, Error::EntryError { code, .. } if *code == 403)
    }

    /// Returns `true` if this error indicates a transport/connection failure.
    pub fn is_connection_error(&self) -> bool {
        matches!(self, Error::Transport(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_signal_not_found() {
        let err = Error::SignalNotFound {
            path: "Vehicle.Speed".to_string(),
        };
        assert!(err.to_string().contains("Vehicle.Speed"));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_error_display_entry_error() {
        let err = Error::EntryError {
            path: "Vehicle.Speed".to_string(),
            code: 403,
            reason: "permission_denied".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Vehicle.Speed"));
        assert!(msg.contains("403"));
        assert!(msg.contains("permission_denied"));
    }

    #[test]
    fn test_error_display_no_value() {
        let err = Error::NoValue {
            path: "Vehicle.Speed".to_string(),
        };
        assert!(err.to_string().contains("Vehicle.Speed"));
        assert!(err.to_string().contains("no value"));
    }

    #[test]
    fn test_error_display_invalid_endpoint() {
        let err = Error::InvalidEndpoint("bad://endpoint".to_string());
        assert!(err.to_string().contains("bad://endpoint"));
    }

    #[test]
    fn test_is_permission_denied_grpc_status() {
        let err = Error::Status(tonic::Status::permission_denied("not allowed"));
        assert!(err.is_permission_denied());
    }

    #[test]
    fn test_is_permission_denied_entry_error_403() {
        let err = Error::EntryError {
            path: "Vehicle.Speed".to_string(),
            code: 403,
            reason: "forbidden".to_string(),
        };
        assert!(err.is_permission_denied());
    }

    #[test]
    fn test_is_not_permission_denied_for_other_errors() {
        let err = Error::SignalNotFound {
            path: "Vehicle.Speed".to_string(),
        };
        assert!(!err.is_permission_denied());

        let err = Error::EntryError {
            path: "Vehicle.Speed".to_string(),
            code: 404,
            reason: "not_found".to_string(),
        };
        assert!(!err.is_permission_denied());
    }

    #[test]
    fn test_is_connection_error() {
        let err = Error::SignalNotFound {
            path: "Vehicle.Speed".to_string(),
        };
        assert!(!err.is_connection_error());

        let err = Error::InvalidEndpoint("bad".to_string());
        assert!(!err.is_connection_error());
    }

    #[test]
    fn test_error_from_tonic_status() {
        let status = tonic::Status::not_found("signal not found");
        let err: Error = status.into();
        assert!(matches!(err, Error::Status(_)));
    }
}
