//! Registry authentication for UPDATE_SERVICE.
//!
//! This module handles OCI registry authentication with Bearer token support,
//! including token caching and refresh.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::debug;

use crate::error::UpdateError;
use crate::logger::{AuthEvent, OperationLogger};

/// Registry credentials.
#[derive(Clone, Debug)]
pub struct RegistryCredentials {
    /// Username for registry authentication
    pub username: String,
    /// Password for registry authentication
    pub password: String,
}

impl RegistryCredentials {
    /// Create new credentials.
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    /// Load credentials from environment variables.
    pub fn from_env() -> Option<Self> {
        let username = std::env::var("REGISTRY_USERNAME").ok()?;
        let password = std::env::var("REGISTRY_PASSWORD").ok()?;
        Some(Self { username, password })
    }
}

/// Cached authentication token.
#[derive(Clone, Debug)]
pub struct CachedToken {
    /// The Bearer token
    pub token: String,
    /// Token expiration time
    pub expires_at: SystemTime,
}

impl CachedToken {
    /// Create a new cached token.
    pub fn new(token: String, expires_in_secs: u64) -> Self {
        Self {
            token,
            expires_at: SystemTime::now() + Duration::from_secs(expires_in_secs),
        }
    }

    /// Check if the token is still valid (with buffer for refresh).
    pub fn is_valid(&self, buffer_secs: u64) -> bool {
        let buffer = Duration::from_secs(buffer_secs);
        SystemTime::now() + buffer < self.expires_at
    }
}

/// Token response from registry.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
    #[serde(default = "default_expires_in")]
    expires_in: u64,
}

fn default_expires_in() -> u64 {
    300 // Default 5 minutes
}

/// Registry authenticator for OCI registries.
#[derive(Clone)]
pub struct RegistryAuthenticator {
    http_client: Client,
    credentials: Option<RegistryCredentials>,
    token_cache: Arc<RwLock<HashMap<String, CachedToken>>>,
    token_refresh_buffer_secs: u64,
    logger: Arc<OperationLogger>,
}

impl RegistryAuthenticator {
    /// Create a new authenticator with optional credentials.
    pub fn new(
        credentials: Option<RegistryCredentials>,
        token_refresh_buffer_secs: u64,
        logger: Arc<OperationLogger>,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            credentials,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            token_refresh_buffer_secs,
            logger,
        }
    }

    /// Create authenticator from environment variables.
    pub fn from_env(logger: Arc<OperationLogger>) -> Self {
        Self::new(RegistryCredentials::from_env(), 60, logger)
    }

    /// Get a valid Bearer token for the registry.
    /// Returns cached token if valid, otherwise fetches new token.
    /// Returns None for anonymous access.
    pub async fn get_token(
        &self,
        registry_url: &str,
        scope: &str,
        correlation_id: &str,
    ) -> Result<Option<String>, UpdateError> {
        let cache_key = format!("{}:{}", registry_url, scope);

        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.is_valid(self.token_refresh_buffer_secs) {
                    debug!("Using cached token for {}", registry_url);
                    return Ok(Some(cached.token.clone()));
                }
            }
        }

        // No credentials means anonymous access
        if self.credentials.is_none() {
            self.logger
                .log_auth_event(correlation_id, registry_url, AuthEvent::AnonymousAccess);
            return Ok(None);
        }

        // Fetch new token
        self.logger
            .log_auth_event(correlation_id, registry_url, AuthEvent::TokenRequested);

        let token = self
            .fetch_token(registry_url, scope, correlation_id)
            .await?;

        // Cache the token
        {
            let mut cache = self.token_cache.write().await;
            cache.insert(cache_key, token.clone());
        }

        self.logger
            .log_auth_event(correlation_id, registry_url, AuthEvent::TokenCached);

        Ok(Some(token.token))
    }

    /// Fetch a new token from the registry token endpoint.
    async fn fetch_token(
        &self,
        registry_url: &str,
        scope: &str,
        correlation_id: &str,
    ) -> Result<CachedToken, UpdateError> {
        let credentials = self.credentials.as_ref().ok_or_else(|| {
            UpdateError::AuthenticationFailed("No credentials configured".to_string())
        })?;

        // Parse registry URL to get host
        let host = extract_host(registry_url)?;
        let token_endpoint = format!("https://{}/v2/token", host);

        debug!("Fetching token from {}", token_endpoint);

        let response = self
            .http_client
            .get(&token_endpoint)
            .query(&[("service", host.as_str()), ("scope", scope)])
            .basic_auth(&credentials.username, Some(&credentials.password))
            .send()
            .await
            .map_err(|e| {
                self.logger.log_auth_event(
                    correlation_id,
                    registry_url,
                    AuthEvent::AuthenticationFailed(e.to_string()),
                );
                UpdateError::TokenEndpointUnreachable(e.to_string())
            })?;

        if response.status() == 401 || response.status() == 403 {
            self.logger.log_auth_event(
                correlation_id,
                registry_url,
                AuthEvent::AuthenticationFailed("Invalid credentials".to_string()),
            );
            return Err(UpdateError::InvalidCredentials);
        }

        if !response.status().is_success() {
            let msg = format!("Token endpoint returned {}", response.status());
            self.logger.log_auth_event(
                correlation_id,
                registry_url,
                AuthEvent::AuthenticationFailed(msg.clone()),
            );
            return Err(UpdateError::AuthenticationFailed(msg));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            UpdateError::AuthenticationFailed(format!("Failed to parse token response: {}", e))
        })?;

        self.logger
            .log_auth_event(correlation_id, registry_url, AuthEvent::TokenObtained);

        Ok(CachedToken::new(
            token_response.token,
            token_response.expires_in,
        ))
    }

    /// Handle a 401 challenge and return the token.
    pub async fn handle_401_challenge(
        &self,
        www_authenticate: &str,
        correlation_id: &str,
    ) -> Result<String, UpdateError> {
        // Parse WWW-Authenticate header
        // Format: Bearer realm="https://registry/v2/token",service="registry",scope="repository:pull"
        let realm = extract_realm(www_authenticate)?;
        let service = extract_param(www_authenticate, "service").unwrap_or_default();
        let scope = extract_param(www_authenticate, "scope").unwrap_or_default();

        debug!(
            "Handling 401 challenge: realm={}, service={}, scope={}",
            realm, service, scope
        );

        let credentials = self.credentials.as_ref().ok_or_else(|| {
            UpdateError::AuthenticationFailed("No credentials for 401 challenge".to_string())
        })?;

        let mut request = self.http_client.get(&realm);

        if !service.is_empty() {
            request = request.query(&[("service", &service)]);
        }
        if !scope.is_empty() {
            request = request.query(&[("scope", &scope)]);
        }

        request = request.basic_auth(&credentials.username, Some(&credentials.password));

        let response = request
            .send()
            .await
            .map_err(|e| UpdateError::TokenEndpointUnreachable(e.to_string()))?;

        if response.status() == 401 || response.status() == 403 {
            return Err(UpdateError::InvalidCredentials);
        }

        if !response.status().is_success() {
            return Err(UpdateError::AuthenticationFailed(format!(
                "Token endpoint returned {}",
                response.status()
            )));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            UpdateError::AuthenticationFailed(format!("Failed to parse token: {}", e))
        })?;

        // Cache it
        let cache_key = format!("{}:{}", service, scope);
        {
            let mut cache = self.token_cache.write().await;
            cache.insert(
                cache_key,
                CachedToken::new(token_response.token.clone(), token_response.expires_in),
            );
        }

        self.logger
            .log_auth_event(correlation_id, &realm, AuthEvent::TokenObtained);

        Ok(token_response.token)
    }

    /// Check if credentials are configured.
    pub fn has_credentials(&self) -> bool {
        self.credentials.is_some()
    }

    /// Clear the token cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.token_cache.write().await;
        cache.clear();
    }
}

/// Extract host from registry URL.
fn extract_host(url: &str) -> Result<String, UpdateError> {
    let url = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = url.split('/').next().unwrap_or(url);
    if host.is_empty() {
        return Err(UpdateError::InvalidRegistryUrl("Empty host".to_string()));
    }
    Ok(host.to_string())
}

/// Extract realm from WWW-Authenticate header.
fn extract_realm(header: &str) -> Result<String, UpdateError> {
    extract_param(header, "realm").ok_or_else(|| {
        UpdateError::AuthenticationFailed("Missing realm in WWW-Authenticate".to_string())
    })
}

/// Extract a parameter from WWW-Authenticate header.
fn extract_param(header: &str, param: &str) -> Option<String> {
    let search = format!("{}=\"", param);
    let start = header.find(&search)?;
    let rest = &header[start + search.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn create_test_authenticator() -> RegistryAuthenticator {
        let logger = Arc::new(OperationLogger::new("test"));
        RegistryAuthenticator::new(None, 60, logger)
    }

    fn create_test_authenticator_with_creds() -> RegistryAuthenticator {
        let logger = Arc::new(OperationLogger::new("test"));
        let creds = RegistryCredentials::new("user".to_string(), "pass".to_string());
        RegistryAuthenticator::new(Some(creds), 60, logger)
    }

    #[test]
    fn test_cached_token_validity() {
        let token = CachedToken::new("test-token".to_string(), 300);
        assert!(token.is_valid(60));

        let expired_token = CachedToken::new("expired".to_string(), 0);
        assert!(!expired_token.is_valid(60));
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(
            extract_host("https://registry.example.com/v2/").unwrap(),
            "registry.example.com"
        );
        assert_eq!(
            extract_host("http://localhost:5000/image").unwrap(),
            "localhost:5000"
        );
        assert_eq!(extract_host("gcr.io/project/image").unwrap(), "gcr.io");
    }

    #[test]
    fn test_extract_param() {
        let header = r#"Bearer realm="https://auth.docker.io/token",service="registry.docker.io",scope="repository:library/nginx:pull""#;

        assert_eq!(
            extract_param(header, "realm"),
            Some("https://auth.docker.io/token".to_string())
        );
        assert_eq!(
            extract_param(header, "service"),
            Some("registry.docker.io".to_string())
        );
        assert_eq!(
            extract_param(header, "scope"),
            Some("repository:library/nginx:pull".to_string())
        );
        assert_eq!(extract_param(header, "missing"), None);
    }

    #[test]
    fn test_has_credentials() {
        let auth_no_creds = create_test_authenticator();
        assert!(!auth_no_creds.has_credentials());

        let auth_with_creds = create_test_authenticator_with_creds();
        assert!(auth_with_creds.has_credentials());
    }

    #[tokio::test]
    async fn test_anonymous_access() {
        let auth = create_test_authenticator();
        let correlation_id = OperationLogger::generate_correlation_id();

        let result = auth
            .get_token(
                "https://registry.example.com",
                "repository:pull",
                &correlation_id,
            )
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Anonymous returns None
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 19: Token Caching and Refresh
        /// Validates: Requirements 11.5
        #[test]
        fn prop_token_caching(
            token in "[a-zA-Z0-9]{32,64}",
            expires_in in 100u64..3600
        ) {
            let cached = CachedToken::new(token.clone(), expires_in);

            // Token should be valid if expires_in > buffer
            if expires_in > 60 {
                prop_assert!(cached.is_valid(60));
            }

            // Token content should be preserved
            prop_assert_eq!(cached.token, token);
        }

        /// Property 21: Anonymous Access for Public Registries
        /// Validates: Requirements 11.7
        #[test]
        fn prop_anonymous_access_no_credentials(
            registry_url in "https://[a-z]+\\.[a-z]+\\.[a-z]+/v2/[a-z]+",
            scope in "repository:[a-z]+:pull"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let auth = create_test_authenticator();
                let correlation_id = OperationLogger::generate_correlation_id();

                let result = auth.get_token(&registry_url, &scope, &correlation_id).await;

                // Should succeed with None (anonymous)
                prop_assert!(result.is_ok());
                prop_assert!(result.unwrap().is_none());
                Ok(())
            })?;
        }

        #[test]
        fn prop_extract_host_valid_urls(
            host in "[a-z]+\\.[a-z]+\\.[a-z]+",
            path in "/v2/[a-z]+"
        ) {
            let url = format!("https://{}{}", host, path);
            let result = extract_host(&url);

            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap(), host);
        }
    }
}
