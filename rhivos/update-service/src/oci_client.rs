//! OCI registry client for pulling container images.
//!
//! Implements 04-REQ-5.1 (OCI image pull), 04-REQ-5.2 (SHA-256 checksum
//! verification against manifest), 04-REQ-5.3 (transition to INSTALLING on
//! checksum match), 04-REQ-5.E1 (ERROR on checksum mismatch), and
//! 04-REQ-5.E2 (ERROR on registry unreachable).
//!
//! The client uses `reqwest` with `rustls-tls` to avoid OpenSSL system
//! dependency. For the demo it fetches the OCI manifest via the distribution
//! API and verifies its SHA-256 digest before proceeding.

use crate::checksum;

/// Trait abstracting OCI registry operations so tests can inject a mock.
#[async_trait::async_trait]
pub trait OciRegistry: Send + Sync + 'static {
    /// Pull the manifest for the given image reference.
    ///
    /// `image_ref` has the form `host:port/name:tag`.
    /// Returns the raw manifest bytes on success.
    async fn pull_manifest(&self, image_ref: &str) -> Result<Vec<u8>, OciError>;

    /// Pull a blob (layer) by digest.
    async fn pull_blob(&self, image_ref: &str, digest: &str) -> Result<Vec<u8>, OciError>;
}

/// Errors that can occur during OCI operations.
#[derive(Debug, Clone)]
pub enum OciError {
    /// The registry is unreachable or returned an HTTP error.
    RegistryUnreachable(String),
    /// The checksum of the pulled manifest does not match.
    ChecksumMismatch {
        expected: String,
        actual: String,
    },
    /// Generic pull failure.
    PullFailed(String),
}

impl std::fmt::Display for OciError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OciError::RegistryUnreachable(msg) => {
                write!(f, "registry unreachable: {}", msg)
            }
            OciError::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "checksum mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            OciError::PullFailed(msg) => write!(f, "pull failed: {}", msg),
        }
    }
}

impl std::error::Error for OciError {}

/// Parse an image_ref like `host:port/name:tag` into (registry_base_url, name, tag).
///
/// Returns `(base_url, name, reference)` where `base_url` is `http://host:port`.
fn parse_image_ref(image_ref: &str) -> Result<(String, String, String), OciError> {
    // Format: host:port/name:tag  or  host/name:tag
    let (host_and_name, tag) = if let Some(idx) = image_ref.rfind(':') {
        // Check if the ':' is part of the host:port or the name:tag
        let after_colon = &image_ref[idx + 1..];
        if after_colon.contains('/') {
            // The ':' is part of host:port, no tag specified
            (image_ref, "latest".to_string())
        } else {
            (&image_ref[..idx], after_colon.to_string())
        }
    } else {
        (image_ref, "latest".to_string())
    };

    // Split host from name at the first '/' after any port
    let slash_idx = host_and_name
        .find('/')
        .ok_or_else(|| OciError::PullFailed(format!("invalid image_ref: {}", image_ref)))?;

    let host = &host_and_name[..slash_idx];
    let name = &host_and_name[slash_idx + 1..];

    if name.is_empty() {
        return Err(OciError::PullFailed(format!(
            "invalid image_ref (empty name): {}",
            image_ref
        )));
    }

    // Use http:// for localhost and plain hostnames (dev/demo context)
    let base_url = format!("http://{}", host);

    Ok((base_url, name.to_string(), tag))
}

/// Verify the SHA-256 checksum of manifest bytes against the expected value.
///
/// Returns `Ok(())` on match or `Err(OciError::ChecksumMismatch)` on mismatch.
pub fn verify_manifest_checksum(
    manifest_bytes: &[u8],
    expected_checksum: &str,
) -> Result<(), OciError> {
    let actual = checksum::compute_sha256(manifest_bytes);
    if checksum::verify_checksum(manifest_bytes, expected_checksum) {
        Ok(())
    } else {
        Err(OciError::ChecksumMismatch {
            expected: expected_checksum.to_string(),
            actual,
        })
    }
}

// ---------------------------------------------------------------------------
// Production implementation using reqwest
// ---------------------------------------------------------------------------

/// Production OCI registry client using HTTP requests.
pub struct HttpOciRegistry {
    client: reqwest::Client,
}

impl Default for HttpOciRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpOciRegistry {
    /// Create a new HTTP-based OCI registry client.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        HttpOciRegistry { client }
    }
}

#[async_trait::async_trait]
impl OciRegistry for HttpOciRegistry {
    async fn pull_manifest(&self, image_ref: &str) -> Result<Vec<u8>, OciError> {
        let (base_url, name, reference) = parse_image_ref(image_ref)?;
        let url = format!("{}/v2/{}/manifests/{}", base_url, name, reference);

        let resp = self.client.get(&url).send().await.map_err(|e| {
            OciError::RegistryUnreachable(format!("{}: {}", url, e))
        })?;

        if !resp.status().is_success() {
            return Err(OciError::PullFailed(format!(
                "manifest fetch returned HTTP {}",
                resp.status()
            )));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| OciError::PullFailed(format!("reading manifest body: {}", e)))
    }

    async fn pull_blob(&self, image_ref: &str, digest: &str) -> Result<Vec<u8>, OciError> {
        let (base_url, name, _) = parse_image_ref(image_ref)?;
        let url = format!("{}/v2/{}/blobs/{}", base_url, name, digest);

        let resp = self.client.get(&url).send().await.map_err(|e| {
            OciError::RegistryUnreachable(format!("{}: {}", url, e))
        })?;

        if !resp.status().is_success() {
            return Err(OciError::PullFailed(format!(
                "blob fetch returned HTTP {}",
                resp.status()
            )));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| OciError::PullFailed(format!("reading blob body: {}", e)))
    }
}

// ---------------------------------------------------------------------------
// Mock implementation for testing
// ---------------------------------------------------------------------------

/// A mock OCI registry for in-process testing.
///
/// Pre-loaded with a manifest that has a known checksum.
pub struct MockOciRegistry {
    /// The manifest bytes to return from `pull_manifest`.
    pub manifest: Vec<u8>,
    /// If true, `pull_manifest` returns `RegistryUnreachable`.
    pub unreachable: bool,
}

impl MockOciRegistry {
    /// Create a mock registry with the given manifest content.
    pub fn new(manifest: &[u8]) -> Self {
        MockOciRegistry {
            manifest: manifest.to_vec(),
            unreachable: false,
        }
    }

    /// Create a mock registry that simulates an unreachable registry.
    pub fn unreachable() -> Self {
        MockOciRegistry {
            manifest: Vec::new(),
            unreachable: true,
        }
    }
}

#[async_trait::async_trait]
impl OciRegistry for MockOciRegistry {
    async fn pull_manifest(&self, _image_ref: &str) -> Result<Vec<u8>, OciError> {
        if self.unreachable {
            return Err(OciError::RegistryUnreachable(
                "mock registry is unreachable".to_string(),
            ));
        }
        Ok(self.manifest.clone())
    }

    async fn pull_blob(&self, _image_ref: &str, _digest: &str) -> Result<Vec<u8>, OciError> {
        if self.unreachable {
            return Err(OciError::RegistryUnreachable(
                "mock registry is unreachable".to_string(),
            ));
        }
        // Return a minimal blob for testing
        Ok(b"mock-layer-content".to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_ref_with_port_and_tag() {
        let (base, name, tag) = parse_image_ref("localhost:5000/adaptor:v1").unwrap();
        assert_eq!(base, "http://localhost:5000");
        assert_eq!(name, "adaptor");
        assert_eq!(tag, "v1");
    }

    #[test]
    fn test_parse_image_ref_with_port_no_tag() {
        let (base, name, tag) = parse_image_ref("localhost:5000/adaptor").unwrap();
        assert_eq!(base, "http://localhost:5000");
        assert_eq!(name, "adaptor");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn test_parse_image_ref_invalid() {
        let result = parse_image_ref("noname");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_manifest_checksum_match() {
        let data = b"test manifest content";
        let expected = checksum::compute_sha256(data);
        assert!(verify_manifest_checksum(data, &expected).is_ok());
    }

    #[test]
    fn test_verify_manifest_checksum_mismatch() {
        let data = b"test manifest content";
        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_manifest_checksum(data, wrong);
        assert!(result.is_err());
        match result.unwrap_err() {
            OciError::ChecksumMismatch { expected, actual } => {
                assert_eq!(expected, wrong);
                assert_eq!(actual, checksum::compute_sha256(data));
            }
            other => panic!("expected ChecksumMismatch, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_mock_registry_pull_manifest() {
        let manifest = b"my test manifest";
        let registry = MockOciRegistry::new(manifest);
        let result = registry
            .pull_manifest("localhost:5000/test:v1")
            .await
            .unwrap();
        assert_eq!(result, manifest.to_vec());
    }

    #[tokio::test]
    async fn test_mock_registry_unreachable() {
        let registry = MockOciRegistry::unreachable();
        let result = registry.pull_manifest("localhost:5000/test:v1").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OciError::RegistryUnreachable(_) => {}
            other => panic!("expected RegistryUnreachable, got: {:?}", other),
        }
    }
}
