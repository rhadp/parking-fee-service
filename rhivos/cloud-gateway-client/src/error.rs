//! Error types for cloud-gateway-client.
//!
//! This module defines all error types used throughout the service,
//! with proper From implementations for error response generation.

use std::path::PathBuf;

use crate::command::{CommandResponse, ResponseStatus};
use chrono::Utc;

/// Top-level error type for the CLOUD_GATEWAY_CLIENT.
#[derive(Debug, thiserror::Error)]
pub enum CloudGatewayError {
    #[error("MQTT error: {0}")]
    Mqtt(#[from] MqttError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Forward error: {0}")]
    Forward(#[from] ForwardError),

    #[error("Telemetry error: {0}")]
    Telemetry(#[from] TelemetryError),

    #[error("Certificate watcher error: {0}")]
    CertWatcher(#[from] CertWatcherError),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Errors during command validation.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Malformed JSON: {0}")]
    MalformedJson(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Authentication failed")]
    AuthFailed,

    #[error("Invalid command type: {0}")]
    InvalidCommandType(String),

    #[error("Invalid door: {0}")]
    InvalidDoor(String),
}

/// Errors during command forwarding to LOCKING_SERVICE.
#[derive(Debug, thiserror::Error)]
pub enum ForwardError {
    #[error("LOCKING_SERVICE unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Command timeout")]
    Timeout,
}

/// Errors during MQTT operations.
#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("TLS error: {0}")]
    TlsError(String),

    #[error("Subscribe failed: {0}")]
    SubscribeFailed(String),

    #[error("Publish failed: {0}")]
    PublishFailed(String),
}

/// Errors during certificate watching/reloading.
#[derive(Debug, thiserror::Error)]
pub enum CertWatcherError {
    #[error("Failed to initialize file watcher: {0}")]
    WatcherInitFailed(String),

    #[error("Failed to watch path {path}: {error}")]
    WatchPathFailed { path: PathBuf, error: String },
}

/// Errors during certificate loading.
#[derive(Debug, thiserror::Error)]
pub enum CertLoadError {
    #[error("Certificate file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Invalid certificate format: {0}")]
    InvalidFormat(String),

    #[error("Certificate parsing failed: {0}")]
    ParseFailed(String),
}

/// Errors during telemetry operations.
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("DATA_BROKER unavailable: {0}")]
    DataBrokerUnavailable(String),

    #[error("Signal subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Publish failed: {0}")]
    PublishFailed(String),
}

impl From<ValidationError> for CommandResponse {
    fn from(err: ValidationError) -> Self {
        let (error_code, error_message) = match &err {
            ValidationError::MalformedJson(msg) => ("MALFORMED_JSON", msg.clone()),
            ValidationError::MissingField(field) => (
                "MISSING_FIELD",
                format!("Missing required field: {}", field),
            ),
            ValidationError::AuthFailed => ("AUTH_FAILED", "Authentication failed".to_string()),
            ValidationError::InvalidCommandType(t) => (
                "INVALID_COMMAND_TYPE",
                format!("Invalid command type: {}", t),
            ),
            ValidationError::InvalidDoor(d) => ("INVALID_DOOR", format!("Invalid door: {}", d)),
        };

        CommandResponse {
            command_id: String::new(), // Will be set by caller if available
            status: ResponseStatus::Failed,
            error_code: Some(error_code.to_string()),
            error_message: Some(error_message),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

impl From<ForwardError> for CommandResponse {
    fn from(err: ForwardError) -> Self {
        let (error_code, error_message) = match &err {
            ForwardError::ServiceUnavailable(msg) => ("SERVICE_UNAVAILABLE", msg.clone()),
            ForwardError::ExecutionFailed(msg) => ("EXECUTION_FAILED", msg.clone()),
            ForwardError::Timeout => ("TIMEOUT", "Command execution timed out".to_string()),
        };

        CommandResponse {
            command_id: String::new(),
            status: ResponseStatus::Failed,
            error_code: Some(error_code.to_string()),
            error_message: Some(error_message),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_to_response() {
        let err = ValidationError::AuthFailed;
        let resp: CommandResponse = err.into();
        assert_eq!(resp.status, ResponseStatus::Failed);
        assert_eq!(resp.error_code.as_deref(), Some("AUTH_FAILED"));
    }

    #[test]
    fn test_forward_error_to_response() {
        let err = ForwardError::Timeout;
        let resp: CommandResponse = err.into();
        assert_eq!(resp.status, ResponseStatus::Failed);
        assert_eq!(resp.error_code.as_deref(), Some("TIMEOUT"));
    }

    #[test]
    fn test_malformed_json_error() {
        let err = ValidationError::MalformedJson("unexpected EOF".to_string());
        let resp: CommandResponse = err.into();
        assert_eq!(resp.error_code.as_deref(), Some("MALFORMED_JSON"));
    }

    #[test]
    fn test_missing_field_error() {
        let err = ValidationError::MissingField("command_id".to_string());
        let resp: CommandResponse = err.into();
        assert_eq!(resp.error_code.as_deref(), Some("MISSING_FIELD"));
        assert!(resp.error_message.unwrap().contains("command_id"));
    }
}
