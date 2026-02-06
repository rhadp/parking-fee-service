//! Test utilities for UPDATE_SERVICE.
//!
//! This module provides mock implementations and test helpers
//! for unit and integration testing.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use tokio::sync::RwLock;

use crate::attestation::{
    Attestation, AttestationPayload, AttestationSignature, AttestationSubject,
};
use crate::authenticator::RegistryAuthenticator;
use crate::config::ServiceConfig;
use crate::downloader::DownloadedImage;
use crate::logger::OperationLogger;

/// Mock registry response.
#[derive(Debug, Clone)]
pub struct MockRegistryResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl MockRegistryResponse {
    /// Create a success response.
    pub fn success(body: &str) -> Self {
        Self {
            status: 200,
            body: body.to_string(),
            headers: HashMap::new(),
        }
    }

    /// Create a 401 unauthorized response.
    pub fn unauthorized(www_authenticate: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("www-authenticate".to_string(), www_authenticate.to_string());
        Self {
            status: 401,
            body: String::new(),
            headers,
        }
    }

    /// Create a 404 not found response.
    pub fn not_found() -> Self {
        Self {
            status: 404,
            body: "Not Found".to_string(),
            headers: HashMap::new(),
        }
    }

    /// Create a 500 error response.
    pub fn server_error(message: &str) -> Self {
        Self {
            status: 500,
            body: message.to_string(),
            headers: HashMap::new(),
        }
    }
}

/// Mock registry for testing.
pub struct MockRegistry {
    responses: RwLock<HashMap<String, MockRegistryResponse>>,
    request_count: RwLock<HashMap<String, usize>>,
}

impl MockRegistry {
    /// Create a new mock registry.
    pub fn new() -> Self {
        Self {
            responses: RwLock::new(HashMap::new()),
            request_count: RwLock::new(HashMap::new()),
        }
    }

    /// Add a response for a path.
    pub async fn add_response(&self, path: &str, response: MockRegistryResponse) {
        let mut responses = self.responses.write().await;
        responses.insert(path.to_string(), response);
    }

    /// Get the number of requests to a path.
    pub async fn request_count(&self, path: &str) -> usize {
        let counts = self.request_count.read().await;
        *counts.get(path).unwrap_or(&0)
    }

    /// Get a response for a path (simulating a request).
    pub async fn get_response(&self, path: &str) -> Option<MockRegistryResponse> {
        // Increment request count
        {
            let mut counts = self.request_count.write().await;
            *counts.entry(path.to_string()).or_insert(0) += 1;
        }

        let responses = self.responses.read().await;
        responses.get(path).cloned()
    }
}

impl Default for MockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a test attestation.
pub fn create_test_attestation(digest: &str) -> Attestation {
    let mut subject_digest = HashMap::new();
    subject_digest.insert("sha256".to_string(), digest.to_string());

    Attestation {
        payload_type: "application/vnd.in-toto+json".to_string(),
        payload: AttestationPayload {
            subject: vec![AttestationSubject {
                name: "test-image".to_string(),
                digest: subject_digest,
            }],
            predicate_type: "https://slsa.dev/provenance/v0.2".to_string(),
            predicate: json!({}),
        },
        signatures: vec![AttestationSignature {
            keyid: "test-key".to_string(),
            sig: "valid-signature-base64".to_string(),
        }],
    }
}

/// Create a test downloaded image.
pub fn create_test_downloaded_image(
    adapter_id: &str,
    temp_dir: &std::path::Path,
) -> DownloadedImage {
    let adapter_dir = temp_dir.join(adapter_id);
    let layers_dir = adapter_dir.join("layers");

    // Create directories
    std::fs::create_dir_all(&layers_dir).ok();

    // Create manifest
    let manifest_path = adapter_dir.join("manifest.json");
    std::fs::write(&manifest_path, r#"{"schemaVersion": 2}"#).ok();

    // Create config
    let config_path = adapter_dir.join("config.json");
    std::fs::write(&config_path, r#"{"architecture": "amd64"}"#).ok();

    DownloadedImage {
        manifest_path,
        layers_dir,
        config_path,
        digest: "sha256:abc123def456".to_string(),
    }
}

/// Create a test service configuration.
pub fn create_test_config(storage_path: &str) -> ServiceConfig {
    ServiceConfig {
        listen_addr: "127.0.0.1:0".to_string(),
        tls_cert_path: String::new(),
        tls_key_path: String::new(),
        storage_path: storage_path.to_string(),
        data_broker_socket: "/tmp/test-databroker.sock".to_string(),
        download_max_retries: 3,
        download_base_delay_ms: 10,
        download_max_delay_ms: 100,
        offload_threshold_hours: 24,
        offload_check_interval_minutes: 60,
        registry_username: None,
        registry_password: None,
        token_refresh_buffer_secs: 60,
        log_level: "debug".to_string(),
    }
}

/// Create a test authenticator with no credentials.
pub fn create_test_authenticator() -> Arc<RegistryAuthenticator> {
    let logger = Arc::new(OperationLogger::new("test"));
    Arc::new(RegistryAuthenticator::new(None, 60, logger))
}

/// Generate test manifests for mock registry.
pub fn generate_manifest_json(config_digest: &str, layer_digest: &str) -> String {
    json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
        "config": {
            "mediaType": "application/vnd.docker.container.image.v1+json",
            "size": 7023,
            "digest": config_digest
        },
        "layers": [
            {
                "mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
                "size": 32654,
                "digest": layer_digest
            }
        ]
    })
    .to_string()
}

/// Generate test referrers response.
pub fn generate_referrers_json(attestation_digest: &str) -> String {
    json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.index.v1+json",
        "manifests": [
            {
                "mediaType": "application/vnd.oci.image.manifest.v1+json",
                "digest": attestation_digest,
                "size": 1024,
                "artifactType": "application/vnd.in-toto+json"
            }
        ]
    })
    .to_string()
}

/// Generate token response.
pub fn generate_token_response(token: &str, expires_in: u64) -> String {
    json!({
        "token": token,
        "expires_in": expires_in
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_registry() {
        let registry = MockRegistry::new();

        registry
            .add_response("/v2/", MockRegistryResponse::success("{}"))
            .await;

        let response = registry.get_response("/v2/").await;
        assert!(response.is_some());
        assert_eq!(response.unwrap().status, 200);
        assert_eq!(registry.request_count("/v2/").await, 1);
    }

    #[test]
    fn test_create_test_attestation() {
        let attestation = create_test_attestation("abc123");

        assert_eq!(attestation.payload_type, "application/vnd.in-toto+json");
        assert!(!attestation.signatures.is_empty());
        assert!(!attestation.payload.subject.is_empty());

        let subject = &attestation.payload.subject[0];
        assert_eq!(subject.digest.get("sha256"), Some(&"abc123".to_string()));
    }

    #[test]
    fn test_create_test_config() {
        let config = create_test_config("/tmp/test");

        assert_eq!(config.storage_path, "/tmp/test");
        assert_eq!(config.download_max_retries, 3);
    }

    #[test]
    fn test_generate_manifest_json() {
        let manifest = generate_manifest_json("sha256:config123", "sha256:layer456");

        assert!(manifest.contains("sha256:config123"));
        assert!(manifest.contains("sha256:layer456"));
    }

    #[test]
    fn test_generate_token_response() {
        let response = generate_token_response("my-token", 300);

        assert!(response.contains("my-token"));
        assert!(response.contains("300"));
    }
}
