//! Error types for UPDATE_SERVICE.
//!
//! This module defines all error types used throughout the service,
//! with proper mapping to gRPC status codes.

use thiserror::Error;
use tonic::Status;

/// Result type for UPDATE_SERVICE operations.
pub type UpdateResult<T> = Result<T, UpdateError>;

/// Error types for UPDATE_SERVICE operations.
#[derive(Debug, Error, Clone)]
pub enum UpdateError {
    /// Download failed after all retries
    #[error("Download failed: {0}")]
    DownloadError(String),

    /// Attestation validation failed
    #[error("Attestation validation failed: {0}")]
    ValidationError(String),

    /// Container operation failed
    #[error("Container operation failed: {0}")]
    ContainerError(String),

    /// Adapter not found
    #[error("Adapter not found: {0}")]
    AdapterNotFound(String),

    /// Adapter already exists
    #[error("Adapter already exists: {0}")]
    AdapterAlreadyExists(String),

    /// Registry unavailable
    #[error("Registry unavailable: {0}")]
    RegistryUnavailable(String),

    /// Invalid registry URL
    #[error("Invalid registry URL: {0}")]
    InvalidRegistryUrl(String),

    /// Attestation subject digest mismatch
    #[error("Attestation subject digest mismatch: expected {expected}, got {actual}")]
    AttestationDigestMismatch { expected: String, actual: String },

    /// Invalid attestation signature
    #[error("Invalid attestation signature")]
    InvalidAttestationSignature,

    /// Missing attestation field
    #[error("Missing attestation field: {0}")]
    MissingAttestationField(String),

    /// Attestation not found for image
    #[error("Attestation not found for image")]
    AttestationNotFound,

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Token endpoint unreachable
    #[error("Token endpoint unreachable: {0}")]
    TokenEndpointUnreachable(String),

    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// State transition error
    #[error("State transition error: {0}")]
    StateError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<UpdateError> for Status {
    fn from(err: UpdateError) -> Self {
        match err {
            UpdateError::InvalidRegistryUrl(msg) => Status::invalid_argument(msg),

            UpdateError::RegistryUnavailable(msg) => Status::unavailable(msg),

            UpdateError::DownloadError(msg) => {
                Status::unavailable(format!("Download failed: {}", msg))
            }

            UpdateError::AttestationDigestMismatch { expected, actual } => {
                Status::failed_precondition(format!(
                    "Attestation subject digest mismatch: expected {}, got {}",
                    expected, actual
                ))
            }

            UpdateError::InvalidAttestationSignature => {
                Status::failed_precondition("Invalid attestation signature")
            }

            UpdateError::MissingAttestationField(field) => {
                Status::failed_precondition(format!("Missing attestation field: {}", field))
            }

            UpdateError::AttestationNotFound => {
                Status::failed_precondition("Attestation not found for image")
            }

            UpdateError::ValidationError(msg) => {
                Status::failed_precondition(format!("Validation failed: {}", msg))
            }

            UpdateError::ContainerError(msg) => {
                Status::internal(format!("Container error: {}", msg))
            }

            UpdateError::AdapterNotFound(id) => {
                Status::not_found(format!("Adapter not found: {}", id))
            }

            UpdateError::AdapterAlreadyExists(id) => {
                Status::already_exists(format!("Adapter already exists: {}", id))
            }

            UpdateError::AuthenticationFailed(msg) => {
                Status::unauthenticated(format!("Authentication failed: {}", msg))
            }

            UpdateError::TokenEndpointUnreachable(msg) => {
                Status::unavailable(format!("Token endpoint unreachable: {}", msg))
            }

            UpdateError::InvalidCredentials => {
                Status::permission_denied("Invalid registry credentials")
            }

            UpdateError::StateError(msg) => Status::internal(format!("State error: {}", msg)),

            UpdateError::InternalError(msg) => Status::internal(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_error_to_status_mapping() {
        let err = UpdateError::AdapterNotFound("test-adapter".to_string());
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);

        let err = UpdateError::InvalidRegistryUrl("bad-url".to_string());
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);

        let err = UpdateError::InvalidCredentials;
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::PermissionDenied);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_error_message_preserved(
            msg in "[a-zA-Z0-9 ]{5,50}"
        ) {
            let err = UpdateError::DownloadError(msg.clone());
            let status: Status = err.into();

            prop_assert!(status.message().contains(&msg));
        }

        #[test]
        fn prop_adapter_not_found_maps_to_not_found(
            adapter_id in "[a-z][a-z0-9-]{3,20}"
        ) {
            let err = UpdateError::AdapterNotFound(adapter_id.clone());
            let status: Status = err.into();

            prop_assert_eq!(status.code(), tonic::Code::NotFound);
            prop_assert!(status.message().contains(&adapter_id));
        }
    }
}
