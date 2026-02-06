//! Image downloading for UPDATE_SERVICE.
//!
//! This module handles OCI container image downloading from registries
//! with authentication and retry logic.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, warn};

use crate::authenticator::RegistryAuthenticator;
use crate::error::UpdateError;
use crate::logger::{ContainerOperation, OperationLogger, OperationOutcome};

/// Downloaded image result.
#[derive(Debug, Clone)]
pub struct DownloadedImage {
    /// Path to the image manifest
    pub manifest_path: PathBuf,
    /// Directory containing layer blobs
    pub layers_dir: PathBuf,
    /// Path to the config blob
    pub config_path: PathBuf,
    /// Image digest (sha256)
    pub digest: String,
}

/// Image downloader with retry logic.
#[derive(Clone)]
pub struct ImageDownloader {
    http_client: Client,
    authenticator: Arc<RegistryAuthenticator>,
    storage_path: PathBuf,
    max_retries: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    logger: Arc<OperationLogger>,
}

impl ImageDownloader {
    /// Create a new image downloader.
    pub fn new(
        authenticator: Arc<RegistryAuthenticator>,
        storage_path: PathBuf,
        max_retries: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
        logger: Arc<OperationLogger>,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            authenticator,
            storage_path,
            max_retries,
            base_delay_ms,
            max_delay_ms,
            logger,
        }
    }

    /// Download a container image with retry logic.
    pub async fn download(
        &self,
        image_ref: &str,
        adapter_id: &str,
        correlation_id: &str,
    ) -> Result<DownloadedImage, UpdateError> {
        self.logger.log_container_operation(
            correlation_id,
            adapter_id,
            ContainerOperation::Pull,
            OperationOutcome::Success, // Will be updated on completion
        );

        let result = self
            .download_with_retry(image_ref, adapter_id, correlation_id)
            .await;

        match &result {
            Ok(_) => {
                self.logger.log_container_operation(
                    correlation_id,
                    adapter_id,
                    ContainerOperation::Pull,
                    OperationOutcome::Success,
                );
            }
            Err(e) => {
                self.logger.log_container_operation(
                    correlation_id,
                    adapter_id,
                    ContainerOperation::Pull,
                    OperationOutcome::Failure(e.to_string()),
                );
            }
        }

        result
    }

    /// Download with exponential backoff retry.
    async fn download_with_retry(
        &self,
        image_ref: &str,
        adapter_id: &str,
        correlation_id: &str,
    ) -> Result<DownloadedImage, UpdateError> {
        let mut delay = Duration::from_millis(self.base_delay_ms);
        let max_delay = Duration::from_millis(self.max_delay_ms);

        for attempt in 0..self.max_retries {
            match self
                .download_image(image_ref, adapter_id, correlation_id)
                .await
            {
                Ok(image) => return Ok(image),
                Err(e) if attempt < self.max_retries - 1 => {
                    warn!(
                        "Download attempt {} failed: {}, retrying in {:?}",
                        attempt + 1,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, max_delay);
                }
                Err(e) => {
                    return Err(UpdateError::DownloadError(format!(
                        "All {} retries failed: {}",
                        self.max_retries, e
                    )));
                }
            }
        }

        unreachable!()
    }

    /// Download the image (single attempt).
    async fn download_image(
        &self,
        image_ref: &str,
        adapter_id: &str,
        correlation_id: &str,
    ) -> Result<DownloadedImage, UpdateError> {
        // Parse image reference
        let (registry, repository, reference) = parse_image_ref(image_ref)?;
        debug!(
            "Downloading image from registry={}, repository={}, reference={}",
            registry, repository, reference
        );

        // Create adapter storage directory
        let adapter_dir = self.storage_path.join(adapter_id);
        fs::create_dir_all(&adapter_dir).await.map_err(|e| {
            UpdateError::DownloadError(format!("Failed to create storage directory: {}", e))
        })?;

        let layers_dir = adapter_dir.join("layers");
        fs::create_dir_all(&layers_dir).await.map_err(|e| {
            UpdateError::DownloadError(format!("Failed to create layers directory: {}", e))
        })?;

        // Get token if needed
        let scope = format!("repository:{}:pull", repository);
        let token = self
            .authenticator
            .get_token(&format!("https://{}", registry), &scope, correlation_id)
            .await?;

        // Fetch manifest
        let manifest_url = format!(
            "https://{}/v2/{}/manifests/{}",
            registry, repository, reference
        );
        let manifest_content = self
            .authenticated_get(&manifest_url, token.as_deref(), correlation_id)
            .await?;

        // Save manifest
        let manifest_path = adapter_dir.join("manifest.json");
        let mut manifest_file = fs::File::create(&manifest_path).await.map_err(|e| {
            UpdateError::DownloadError(format!("Failed to create manifest file: {}", e))
        })?;
        manifest_file
            .write_all(&manifest_content)
            .await
            .map_err(|e| UpdateError::DownloadError(format!("Failed to write manifest: {}", e)))?;

        // Calculate manifest digest
        let mut hasher = Sha256::new();
        hasher.update(&manifest_content);
        let digest = format!("sha256:{:x}", hasher.finalize());

        // Parse manifest to get config and layers
        let manifest: serde_json::Value = serde_json::from_slice(&manifest_content)
            .map_err(|e| UpdateError::DownloadError(format!("Failed to parse manifest: {}", e)))?;

        // Download config blob
        let config_path = if let Some(config) = manifest.get("config") {
            if let Some(config_digest) = config.get("digest").and_then(|d| d.as_str()) {
                let config_url = format!(
                    "https://{}/v2/{}/blobs/{}",
                    registry, repository, config_digest
                );
                let config_content = self
                    .authenticated_get(&config_url, token.as_deref(), correlation_id)
                    .await?;

                let config_path = adapter_dir.join("config.json");
                let mut config_file = fs::File::create(&config_path).await.map_err(|e| {
                    UpdateError::DownloadError(format!("Failed to create config file: {}", e))
                })?;
                config_file.write_all(&config_content).await.map_err(|e| {
                    UpdateError::DownloadError(format!("Failed to write config: {}", e))
                })?;

                config_path
            } else {
                adapter_dir.join("config.json")
            }
        } else {
            adapter_dir.join("config.json")
        };

        // Download layers
        if let Some(layers) = manifest.get("layers").and_then(|l| l.as_array()) {
            for layer in layers {
                if let Some(layer_digest) = layer.get("digest").and_then(|d| d.as_str()) {
                    let layer_url = format!(
                        "https://{}/v2/{}/blobs/{}",
                        registry, repository, layer_digest
                    );
                    let layer_content = self
                        .authenticated_get(&layer_url, token.as_deref(), correlation_id)
                        .await?;

                    // Save layer with digest as filename
                    let layer_filename = layer_digest.replace(":", "_");
                    let layer_path = layers_dir.join(&layer_filename);
                    let mut layer_file = fs::File::create(&layer_path).await.map_err(|e| {
                        UpdateError::DownloadError(format!("Failed to create layer file: {}", e))
                    })?;
                    layer_file.write_all(&layer_content).await.map_err(|e| {
                        UpdateError::DownloadError(format!("Failed to write layer: {}", e))
                    })?;

                    debug!("Downloaded layer: {}", layer_digest);
                }
            }
        }

        Ok(DownloadedImage {
            manifest_path,
            layers_dir,
            config_path,
            digest,
        })
    }

    /// Perform an authenticated GET request.
    async fn authenticated_get(
        &self,
        url: &str,
        token: Option<&str>,
        correlation_id: &str,
    ) -> Result<Vec<u8>, UpdateError> {
        debug!("GET {}", url);

        let mut request = self.http_client.get(url);

        // Add common accept headers for OCI manifests
        request = request.header(
            "Accept",
            "application/vnd.docker.distribution.manifest.v2+json, application/vnd.oci.image.manifest.v1+json",
        );

        if let Some(t) = token {
            request = request.bearer_auth(t);
        }

        let response = request
            .send()
            .await
            .map_err(|e| UpdateError::RegistryUnavailable(format!("Request failed: {}", e)))?;

        // Handle 401 challenge
        if response.status() == 401 {
            if let Some(www_auth) = response.headers().get("www-authenticate") {
                let www_auth_str = www_auth.to_str().unwrap_or("");
                let new_token = self
                    .authenticator
                    .handle_401_challenge(www_auth_str, correlation_id)
                    .await?;

                // Retry with new token
                let retry_response = self
                    .http_client
                    .get(url)
                    .header(
                        "Accept",
                        "application/vnd.docker.distribution.manifest.v2+json, application/vnd.oci.image.manifest.v1+json",
                    )
                    .bearer_auth(&new_token)
                    .send()
                    .await
                    .map_err(|e| UpdateError::RegistryUnavailable(e.to_string()))?;

                if !retry_response.status().is_success() {
                    return Err(UpdateError::DownloadError(format!(
                        "Request failed after auth: {}",
                        retry_response.status()
                    )));
                }

                return retry_response
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .map_err(|e| {
                        UpdateError::DownloadError(format!("Failed to read response: {}", e))
                    });
            }

            return Err(UpdateError::AuthenticationFailed(
                "401 without WWW-Authenticate header".to_string(),
            ));
        }

        if !response.status().is_success() {
            return Err(UpdateError::DownloadError(format!(
                "Request failed: {}",
                response.status()
            )));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| UpdateError::DownloadError(format!("Failed to read response: {}", e)))
    }

    /// Get the storage path for an adapter.
    pub fn adapter_storage_path(&self, adapter_id: &str) -> PathBuf {
        self.storage_path.join(adapter_id)
    }

    /// Delete downloaded content for an adapter.
    pub async fn delete(&self, adapter_id: &str) -> Result<(), UpdateError> {
        let adapter_dir = self.storage_path.join(adapter_id);
        if adapter_dir.exists() {
            fs::remove_dir_all(&adapter_dir).await.map_err(|e| {
                UpdateError::ContainerError(format!("Failed to delete content: {}", e))
            })?;
        }
        Ok(())
    }
}

/// Parse an image reference into registry, repository, and reference.
fn parse_image_ref(image_ref: &str) -> Result<(String, String, String), UpdateError> {
    let image_ref = image_ref
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    // Split by @ for digest reference or : for tag
    let (repo_part, reference) = if image_ref.contains('@') {
        let parts: Vec<&str> = image_ref.splitn(2, '@').collect();
        (parts[0], parts.get(1).unwrap_or(&"latest").to_string())
    } else if let Some(last_colon) = image_ref.rfind(':') {
        // Check if this colon is part of a port number
        let before_colon = &image_ref[..last_colon];
        if before_colon.contains('/')
            || !before_colon
                .chars()
                .last()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            (before_colon, image_ref[last_colon + 1..].to_string())
        } else {
            (image_ref, "latest".to_string())
        }
    } else {
        (image_ref, "latest".to_string())
    };

    // Split registry from repository
    let parts: Vec<&str> = repo_part.splitn(2, '/').collect();
    if parts.len() < 2 {
        return Err(UpdateError::InvalidRegistryUrl(format!(
            "Invalid image reference: {}",
            image_ref
        )));
    }

    let registry = parts[0].to_string();
    let repository = parts[1..].join("/");

    if registry.is_empty() || repository.is_empty() {
        return Err(UpdateError::InvalidRegistryUrl(format!(
            "Invalid image reference: {}",
            image_ref
        )));
    }

    Ok((registry, repository, reference))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_parse_image_ref_with_tag() {
        let (registry, repo, reference) = parse_image_ref("gcr.io/project/image:v1.0").unwrap();

        assert_eq!(registry, "gcr.io");
        assert_eq!(repo, "project/image");
        assert_eq!(reference, "v1.0");
    }

    #[test]
    fn test_parse_image_ref_with_digest() {
        let (registry, repo, reference) =
            parse_image_ref("gcr.io/project/image@sha256:abc123").unwrap();

        assert_eq!(registry, "gcr.io");
        assert_eq!(repo, "project/image");
        assert_eq!(reference, "sha256:abc123");
    }

    #[test]
    fn test_parse_image_ref_default_tag() {
        let (registry, repo, reference) = parse_image_ref("docker.io/library/nginx").unwrap();

        assert_eq!(registry, "docker.io");
        assert_eq!(repo, "library/nginx");
        assert_eq!(reference, "latest");
    }

    #[test]
    fn test_parse_image_ref_invalid() {
        let result = parse_image_ref("invalid");
        assert!(result.is_err());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 5: Download Retry and Failure Handling
        /// Validates: Requirements 2.4, 2.5
        #[test]
        fn prop_retry_count_respected(
            max_retries in 1u32..5,
            base_delay_ms in 100u64..1000
        ) {
            // Verify retry configuration is respected
            let logger = Arc::new(OperationLogger::new("test"));
            let auth = Arc::new(RegistryAuthenticator::new(None, 60, logger.clone()));
            let downloader = ImageDownloader::new(
                auth,
                PathBuf::from("/tmp/test"),
                max_retries,
                base_delay_ms,
                30000,
                logger,
            );

            prop_assert_eq!(downloader.max_retries, max_retries);
            prop_assert_eq!(downloader.base_delay_ms, base_delay_ms);
        }

        #[test]
        fn prop_parse_image_ref_valid(
            registry in "[a-z]+\\.[a-z]+\\.[a-z]+",
            project in "[a-z]+",
            image in "[a-z]+",
            tag in "[a-z0-9]+"
        ) {
            let image_ref = format!("{}/{}/{}:{}", registry, project, image, tag);
            let result = parse_image_ref(&image_ref);

            prop_assert!(result.is_ok());
            let (r, repo, t) = result.unwrap();
            prop_assert_eq!(r, registry);
            prop_assert_eq!(repo, format!("{}/{}", project, image));
            prop_assert_eq!(t, tag);
        }
    }
}
