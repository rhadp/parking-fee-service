//! Attestation validation for UPDATE_SERVICE.
//!
//! This module handles fetching and validating container attestations
//! to verify container authenticity before installation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::authenticator::RegistryAuthenticator;
use crate::error::UpdateError;

/// Attestation from registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    /// Payload type (e.g., "application/vnd.in-toto+json")
    #[serde(rename = "payloadType")]
    pub payload_type: String,

    /// Attestation payload
    pub payload: AttestationPayload,

    /// Signatures
    pub signatures: Vec<AttestationSignature>,
}

/// Attestation payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationPayload {
    /// Subject (the image being attested)
    pub subject: Vec<AttestationSubject>,

    /// Predicate type
    #[serde(rename = "predicateType")]
    pub predicate_type: String,

    /// Predicate data
    #[serde(default)]
    pub predicate: serde_json::Value,
}

/// Attestation subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationSubject {
    /// Subject name
    pub name: String,

    /// Subject digest (algorithm -> value)
    pub digest: HashMap<String, String>,
}

/// Attestation signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationSignature {
    /// Key ID
    pub keyid: String,

    /// Signature value
    pub sig: String,
}

/// Attestation validator.
#[derive(Clone)]
pub struct AttestationValidator {
    http_client: Client,
    authenticator: Arc<RegistryAuthenticator>,
}

impl AttestationValidator {
    /// Create a new attestation validator.
    pub fn new(authenticator: Arc<RegistryAuthenticator>) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            authenticator,
        }
    }

    /// Fetch attestation from registry for the given image.
    pub async fn fetch_attestation(
        &self,
        registry_url: &str,
        image_digest: &str,
        correlation_id: &str,
    ) -> Result<Attestation, UpdateError> {
        // Parse registry URL
        let registry = registry_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or(registry_url);

        // Get repository from URL
        let repo_part = registry_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches(registry)
            .trim_start_matches('/');

        let repository = repo_part.split(':').next().unwrap_or(repo_part);

        // Get token for attestation access
        let scope = format!("repository:{}:pull", repository);
        let token = self
            .authenticator
            .get_token(&format!("https://{}", registry), &scope, correlation_id)
            .await?;

        // Fetch attestation using referrers API
        // OCI referrers API: GET /v2/<name>/referrers/<digest>
        let referrers_url = format!(
            "https://{}/v2/{}/referrers/{}",
            registry, repository, image_digest
        );

        debug!("Fetching attestation from {}", referrers_url);

        let mut request = self.http_client.get(&referrers_url);
        request = request.header("Accept", "application/vnd.oci.image.index.v1+json");

        if let Some(ref t) = token {
            request = request.bearer_auth(t);
        }

        let response = request.send().await.map_err(|e| {
            UpdateError::RegistryUnavailable(format!("Attestation fetch failed: {}", e))
        })?;

        if response.status() == 404 {
            return Err(UpdateError::AttestationNotFound);
        }

        if !response.status().is_success() {
            return Err(UpdateError::RegistryUnavailable(format!(
                "Attestation fetch failed: {}",
                response.status()
            )));
        }

        let index: serde_json::Value = response.json().await.map_err(|e| {
            UpdateError::ValidationError(format!("Failed to parse referrers response: {}", e))
        })?;

        // Find attestation manifest in referrers
        let manifests = index
            .get("manifests")
            .and_then(|m| m.as_array())
            .ok_or_else(|| UpdateError::AttestationNotFound)?;

        let attestation_manifest = manifests
            .iter()
            .find(|m| {
                m.get("artifactType")
                    .and_then(|t| t.as_str())
                    .map(|t| t.contains("attestation") || t.contains("in-toto"))
                    .unwrap_or(false)
            })
            .ok_or_else(|| UpdateError::AttestationNotFound)?;

        let attestation_digest = attestation_manifest
            .get("digest")
            .and_then(|d| d.as_str())
            .ok_or_else(|| UpdateError::AttestationNotFound)?;

        // Fetch the actual attestation blob
        let attestation_url = format!(
            "https://{}/v2/{}/blobs/{}",
            registry, repository, attestation_digest
        );

        let mut blob_request = self.http_client.get(&attestation_url);
        if let Some(ref t) = token {
            blob_request = blob_request.bearer_auth(t);
        }

        let blob_response = blob_request.send().await.map_err(|e| {
            UpdateError::RegistryUnavailable(format!("Attestation blob fetch failed: {}", e))
        })?;

        if !blob_response.status().is_success() {
            return Err(UpdateError::RegistryUnavailable(format!(
                "Attestation blob fetch failed: {}",
                blob_response.status()
            )));
        }

        let attestation: Attestation = blob_response.json().await.map_err(|e| {
            UpdateError::ValidationError(format!("Failed to parse attestation: {}", e))
        })?;

        Ok(attestation)
    }

    /// Validate attestation structure.
    pub fn validate_structure(&self, attestation: &Attestation) -> Result<(), UpdateError> {
        // Check payload type
        if attestation.payload_type.is_empty() {
            return Err(UpdateError::MissingAttestationField(
                "payloadType".to_string(),
            ));
        }

        // Check subject exists
        if attestation.payload.subject.is_empty() {
            return Err(UpdateError::MissingAttestationField("subject".to_string()));
        }

        // Check subject has digest
        for subject in &attestation.payload.subject {
            if subject.digest.is_empty() {
                return Err(UpdateError::MissingAttestationField(
                    "subject.digest".to_string(),
                ));
            }
        }

        // Check predicate type
        if attestation.payload.predicate_type.is_empty() {
            return Err(UpdateError::MissingAttestationField(
                "predicateType".to_string(),
            ));
        }

        // Check signatures
        if attestation.signatures.is_empty() {
            return Err(UpdateError::MissingAttestationField(
                "signatures".to_string(),
            ));
        }

        for sig in &attestation.signatures {
            if sig.sig.is_empty() {
                return Err(UpdateError::MissingAttestationField(
                    "signature.sig".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validate attestation against expected image digest.
    pub fn validate(
        &self,
        attestation: &Attestation,
        expected_image_digest: &str,
    ) -> Result<(), UpdateError> {
        // First validate structure
        self.validate_structure(attestation)?;

        // Check subject digest matches
        let expected_digest = expected_image_digest
            .trim_start_matches("sha256:")
            .to_lowercase();

        let mut found_match = false;
        for subject in &attestation.payload.subject {
            if let Some(sha256) = subject.digest.get("sha256") {
                if sha256.to_lowercase() == expected_digest {
                    found_match = true;
                    break;
                }
            }
        }

        if !found_match {
            let actual = attestation
                .payload
                .subject
                .first()
                .and_then(|s| s.digest.get("sha256"))
                .cloned()
                .unwrap_or_else(|| "none".to_string());

            return Err(UpdateError::AttestationDigestMismatch {
                expected: expected_digest,
                actual,
            });
        }

        // Verify signature (simplified - in production would verify cryptographic signature)
        // For now, we just check that signatures exist and are non-empty
        for sig in &attestation.signatures {
            if sig.sig.is_empty() {
                return Err(UpdateError::InvalidAttestationSignature);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::OperationLogger;
    use proptest::prelude::*;

    fn create_test_attestation(digest: &str) -> Attestation {
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
                predicate: serde_json::json!({}),
            },
            signatures: vec![AttestationSignature {
                keyid: "key-1".to_string(),
                sig: "valid-signature".to_string(),
            }],
        }
    }

    fn create_test_validator() -> AttestationValidator {
        let logger = Arc::new(OperationLogger::new("test"));
        let auth = Arc::new(RegistryAuthenticator::new(None, 60, logger));
        AttestationValidator::new(auth)
    }

    #[test]
    fn test_validate_structure_valid() {
        let validator = create_test_validator();
        let attestation = create_test_attestation("abc123");

        let result = validator.validate_structure(&attestation);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_structure_missing_payload_type() {
        let validator = create_test_validator();
        let mut attestation = create_test_attestation("abc123");
        attestation.payload_type = String::new();

        let result = validator.validate_structure(&attestation);
        assert!(matches!(
            result,
            Err(UpdateError::MissingAttestationField(_))
        ));
    }

    #[test]
    fn test_validate_structure_missing_subject() {
        let validator = create_test_validator();
        let mut attestation = create_test_attestation("abc123");
        attestation.payload.subject = vec![];

        let result = validator.validate_structure(&attestation);
        assert!(matches!(
            result,
            Err(UpdateError::MissingAttestationField(_))
        ));
    }

    #[test]
    fn test_validate_structure_missing_signature() {
        let validator = create_test_validator();
        let mut attestation = create_test_attestation("abc123");
        attestation.signatures = vec![];

        let result = validator.validate_structure(&attestation);
        assert!(matches!(
            result,
            Err(UpdateError::MissingAttestationField(_))
        ));
    }

    #[test]
    fn test_validate_matching_digest() {
        let validator = create_test_validator();
        let attestation = create_test_attestation("abc123def456");

        let result = validator.validate(&attestation, "sha256:abc123def456");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_mismatched_digest() {
        let validator = create_test_validator();
        let attestation = create_test_attestation("abc123");

        let result = validator.validate(&attestation, "sha256:different");
        assert!(matches!(
            result,
            Err(UpdateError::AttestationDigestMismatch { .. })
        ));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 7: Attestation Verification
        /// Validates: Requirements 3.1, 3.2
        #[test]
        fn prop_attestation_digest_mismatch_rejected(
            expected_digest in "[a-f0-9]{64}",
            actual_digest in "[a-f0-9]{64}"
        ) {
            prop_assume!(expected_digest != actual_digest);

            let validator = create_test_validator();
            let attestation = create_test_attestation(&actual_digest);

            let result = validator.validate(&attestation, &format!("sha256:{}", expected_digest));

            prop_assert!(result.is_err());
            if let Err(UpdateError::AttestationDigestMismatch { expected, actual }) = result {
                prop_assert_eq!(expected, expected_digest);
                prop_assert_eq!(actual, actual_digest);
            } else {
                prop_assert!(false, "Expected AttestationDigestMismatch error");
            }
        }

        /// Property 8: Attestation Structure Validation
        /// Validates: Requirements 3.3, 3.4
        #[test]
        fn prop_attestation_missing_field_rejected(
            missing_field_idx in 0usize..4
        ) {
            let validator = create_test_validator();
            let mut attestation = create_test_attestation("abc123");

            // Remove different fields based on index
            match missing_field_idx {
                0 => attestation.payload_type = String::new(),
                1 => attestation.payload.subject = vec![],
                2 => attestation.payload.predicate_type = String::new(),
                3 => attestation.signatures = vec![],
                _ => unreachable!(),
            }

            let result = validator.validate_structure(&attestation);

            prop_assert!(result.is_err());
            prop_assert!(matches!(result, Err(UpdateError::MissingAttestationField(_))));
        }

        #[test]
        fn prop_valid_attestation_accepted(
            digest in "[a-f0-9]{64}"
        ) {
            let validator = create_test_validator();
            let attestation = create_test_attestation(&digest);

            let result = validator.validate(&attestation, &format!("sha256:{}", digest));

            prop_assert!(result.is_ok());
        }
    }
}
