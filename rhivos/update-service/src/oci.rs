use std::fmt;

/// Errors from OCI image operations.
#[derive(Debug, Clone)]
pub enum OciError {
    /// Registry is unreachable.
    RegistryUnavailable(String),
    /// Checksum does not match.
    ChecksumMismatch { expected: String, actual: String },
    /// Other pull error.
    PullFailed(String),
}

impl fmt::Display for OciError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OciError::RegistryUnavailable(msg) => {
                write!(f, "failed to pull image: {msg}")
            }
            OciError::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch: expected {expected}, got {actual}")
            }
            OciError::PullFailed(msg) => write!(f, "pull failed: {msg}"),
        }
    }
}

impl std::error::Error for OciError {}

/// Result of a successful image pull.
#[derive(Debug, Clone)]
pub struct PullResult {
    pub digest: String,
}

/// Trait for OCI image pulling and removal.
#[mockall::automock]
#[async_trait::async_trait]
pub trait OciPuller: Send + Sync {
    /// Pull an OCI image and return its manifest digest.
    async fn pull_image(&self, image_ref: &str) -> Result<PullResult, OciError>;

    /// Remove a previously pulled OCI image.
    async fn remove_image(&self, image_ref: &str) -> Result<(), OciError>;
}

/// Verify that a digest matches the expected SHA-256 checksum.
///
/// The `digest` is the OCI manifest digest string (e.g., "sha256:abc123...").
/// The `expected_checksum` is the expected SHA-256 hex string.
pub fn verify_checksum(_digest: &str, _expected_checksum: &str) -> Result<(), OciError> {
    // Stub: not yet implemented
    todo!("checksum verification not yet implemented")
}
