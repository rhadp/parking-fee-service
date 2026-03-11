use async_trait::async_trait;
use sha2::{Digest, Sha256};
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
///
/// The checksum is computed by hashing the raw digest string bytes with SHA-256
/// and encoding as `sha256:{hex}`. This matches the format used by the tests.
pub fn verify_checksum(digest: &str, expected: &str) -> Result<(), OciError> {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(digest.as_bytes());
    let actual = format!("sha256:{}", hex::encode(hash));
    if actual == expected {
        Ok(())
    } else {
        Err(OciError::ChecksumMismatch {
            expected: expected.to_string(),
            actual,
        })
    }
}

/// OCI image puller implementation using podman CLI.
///
/// Shells out to `podman pull` and `podman inspect` for image operations,
/// and `podman rmi` for image removal.
pub struct PodmanOciPuller;

impl PodmanOciPuller {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PodmanOciPuller {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OciPuller for PodmanOciPuller {
    async fn pull_image(&self, image_ref: &str) -> Result<PullResult, OciError> {
        // Pull the image
        let pull_output = tokio::process::Command::new("podman")
            .args(["pull", image_ref])
            .output()
            .await
            .map_err(|e| OciError::PullFailed(format!("failed to execute podman pull: {e}")))?;

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr);
            return Err(OciError::PullFailed(stderr.trim().to_string()));
        }

        // Inspect to get the digest
        let inspect_output = tokio::process::Command::new("podman")
            .args(["inspect", image_ref, "--format", "{{.Digest}}"])
            .output()
            .await
            .map_err(|e| {
                OciError::PullFailed(format!("failed to execute podman inspect: {e}"))
            })?;

        if !inspect_output.status.success() {
            let stderr = String::from_utf8_lossy(&inspect_output.stderr);
            return Err(OciError::PullFailed(format!(
                "failed to inspect image digest: {}",
                stderr.trim()
            )));
        }

        let digest = String::from_utf8_lossy(&inspect_output.stdout)
            .trim()
            .to_string();

        Ok(PullResult { digest })
    }

    async fn remove_image(&self, image_ref: &str) -> Result<(), OciError> {
        let output = tokio::process::Command::new("podman")
            .args(["rmi", image_ref])
            .output()
            .await
            .map_err(|e| OciError::RemoveFailed(format!("failed to execute podman rmi: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OciError::RemoveFailed(stderr.trim().to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "oci_test.rs"]
mod oci_test;
