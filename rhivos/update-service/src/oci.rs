use sha2::{Digest, Sha256};
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
/// The `expected_checksum` is the expected SHA-256 value, formatted as "sha256:<hex>".
///
/// Computes SHA-256 of the entire digest string and compares with `expected_checksum`.
pub fn verify_checksum(digest: &str, expected_checksum: &str) -> Result<(), OciError> {
    let hash = Sha256::digest(digest.as_bytes());
    let actual = format!("sha256:{}", hex::encode(hash));

    if actual == expected_checksum {
        Ok(())
    } else {
        Err(OciError::ChecksumMismatch {
            expected: expected_checksum.to_string(),
            actual,
        })
    }
}

/// OCI image puller backed by the podman CLI.
///
/// Executes `podman pull` and `podman inspect` via `tokio::process::Command`.
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

#[async_trait::async_trait]
impl OciPuller for PodmanOciPuller {
    /// Pull an OCI image using `podman pull`, then extract the manifest digest
    /// via `podman inspect`.
    async fn pull_image(&self, image_ref: &str) -> Result<PullResult, OciError> {
        // Step 1: podman pull <image_ref>
        let pull_output = tokio::process::Command::new("podman")
            .args(["pull", image_ref])
            .output()
            .await
            .map_err(|e| OciError::RegistryUnavailable(e.to_string()))?;

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr);
            // Distinguish registry-unreachable from other pull errors
            if stderr.contains("connection refused")
                || stderr.contains("no such host")
                || stderr.contains("timeout")
                || stderr.contains("unreachable")
            {
                return Err(OciError::RegistryUnavailable(stderr.trim().to_string()));
            }
            return Err(OciError::PullFailed(stderr.trim().to_string()));
        }

        // Step 2: podman inspect <image_ref> --format '{{.Digest}}'
        let inspect_output = tokio::process::Command::new("podman")
            .args(["inspect", image_ref, "--format", "{{.Digest}}"])
            .output()
            .await
            .map_err(|e| OciError::PullFailed(format!("inspect failed: {e}")))?;

        if !inspect_output.status.success() {
            let stderr = String::from_utf8_lossy(&inspect_output.stderr);
            return Err(OciError::PullFailed(format!(
                "inspect failed: {}",
                stderr.trim()
            )));
        }

        let digest = String::from_utf8_lossy(&inspect_output.stdout)
            .trim()
            .to_string();

        if digest.is_empty() {
            return Err(OciError::PullFailed(
                "inspect returned empty digest".to_string(),
            ));
        }

        Ok(PullResult { digest })
    }

    /// Remove a pulled OCI image using `podman rmi`.
    async fn remove_image(&self, image_ref: &str) -> Result<(), OciError> {
        let output = tokio::process::Command::new("podman")
            .args(["rmi", image_ref])
            .output()
            .await
            .map_err(|e| OciError::PullFailed(format!("rmi failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OciError::PullFailed(format!(
                "rmi failed: {}",
                stderr.trim()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-07-3: Valid checksum passes verification.
    #[test]
    fn test_verify_checksum_valid() {
        let digest =
            "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let hash = Sha256::digest(digest.as_bytes());
        let expected = format!("sha256:{}", hex::encode(hash));

        let result = verify_checksum(digest, &expected);
        assert!(result.is_ok(), "valid checksum should pass verification");
    }

    /// TS-07-E1: Invalid checksum fails verification with ChecksumMismatch.
    #[test]
    fn test_verify_checksum_mismatch() {
        let digest =
            "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let expected =
            "sha256:badhash0000000000000000000000000000000000000000000000000000000000";

        let result = verify_checksum(digest, expected);
        assert!(result.is_err(), "mismatched checksum should fail");
        match result.unwrap_err() {
            OciError::ChecksumMismatch {
                expected: e,
                actual: a,
            } => {
                assert_eq!(e, expected);
                assert!(a.starts_with("sha256:"), "actual should be sha256-prefixed");
                assert_ne!(a, expected, "actual should differ from expected");
            }
            other => panic!("expected ChecksumMismatch, got: {:?}", other),
        }
    }

    /// Empty digest still produces a deterministic checksum.
    #[test]
    fn test_verify_checksum_empty_digest() {
        let digest = "";
        let hash = Sha256::digest(digest.as_bytes());
        let expected = format!("sha256:{}", hex::encode(hash));

        let result = verify_checksum(digest, &expected);
        assert!(
            result.is_ok(),
            "empty digest with matching checksum should pass"
        );
    }

    /// Verify PodmanOciPuller can be constructed.
    #[test]
    fn test_podman_oci_puller_new() {
        let _puller = PodmanOciPuller::new();
        let _puller_default = PodmanOciPuller::default();
    }
}
