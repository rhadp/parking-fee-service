//! Error types for RHIVOS services

use thiserror::Error;

/// Result type alias using the shared Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Common error types for RHIVOS services
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// gRPC communication error
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Service not ready
    #[error("Service not ready: {0}")]
    ServiceNotReady(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// TLS error
    #[error("TLS error: {0}")]
    TlsError(String),
}

impl From<Error> for tonic::Status {
    fn from(err: Error) -> Self {
        match err {
            Error::Config(msg) => tonic::Status::failed_precondition(msg),
            Error::Grpc(status) => status,
            Error::Io(e) => tonic::Status::unavailable(e.to_string()),
            Error::Serialization(e) => tonic::Status::invalid_argument(e.to_string()),
            Error::ServiceNotReady(msg) => tonic::Status::failed_precondition(msg),
            Error::InvalidRequest(msg) => tonic::Status::invalid_argument(msg),
            Error::ConnectionFailed(msg) => tonic::Status::unavailable(msg),
            Error::TlsError(msg) => tonic::Status::unavailable(msg),
        }
    }
}
