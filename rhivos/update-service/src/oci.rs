use async_trait::async_trait;
use thiserror::Error;

#[cfg(test)]
use mockall::automock;

/// Errors that can occur during OCI image operations.
#[derive(Debug, Error)]
pub enum OciError {
    #[error("failed to pull image: {0}")]
    PullFailed(String),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("failed to remove image: {0}")]
    RemoveFailed(String),
}

/// Result of a successful image pull.
#[derive(Debug, Clone)]
pub struct PullResult {
    /// The OCI manifest digest (e.g., "sha256:abc123...")
    pub digest: String,
}

/// Trait for OCI image pull and verification operations.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait OciPuller: Send + Sync {
    /// Pull an OCI image by reference. Returns the manifest digest.
    async fn pull_image(&self, image_ref: &str) -> Result<PullResult, OciError>;

    /// Remove a previously pulled OCI image.
    async fn remove_image(&self, image_ref: &str) -> Result<(), OciError>;
}

/// Verify that the SHA-256 hash of the digest string matches the expected checksum.
pub fn verify_checksum(_digest: &str, _expected: &str) -> Result<(), OciError> {
    // Stub: always returns error. Implementation in task group 3.
    Err(OciError::ChecksumMismatch {
        expected: _expected.to_string(),
        actual: "not-implemented".to_string(),
    })
}
