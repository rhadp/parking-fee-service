//! Configuration for UPDATE_SERVICE.
//!
//! This module defines the service configuration with sensible defaults
//! and environment variable loading support.

use serde::{Deserialize, Serialize};

/// Service configuration loaded from environment/file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// TCP address for gRPC server (e.g., "0.0.0.0:50052")
    pub listen_addr: String,

    /// TLS certificate path
    pub tls_cert_path: String,

    /// TLS key path
    pub tls_key_path: String,

    /// Container storage path
    pub storage_path: String,

    /// DATA_BROKER UDS socket path for container networking
    pub data_broker_socket: String,

    /// Max retries for image download
    pub download_max_retries: u32,

    /// Base delay for exponential backoff (ms)
    pub download_base_delay_ms: u64,

    /// Maximum delay for exponential backoff (ms)
    pub download_max_delay_ms: u64,

    /// Offload threshold (hours)
    pub offload_threshold_hours: u64,

    /// Offload check interval (minutes)
    pub offload_check_interval_minutes: u64,

    /// Registry username (from REGISTRY_USERNAME env var)
    pub registry_username: Option<String>,

    /// Registry password (from REGISTRY_PASSWORD env var)
    pub registry_password: Option<String>,

    /// Token cache TTL buffer (seconds before expiry to refresh)
    pub token_refresh_buffer_secs: u64,

    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:50052".to_string(),
            tls_cert_path: "/etc/rhivos/certs/update-service.crt".to_string(),
            tls_key_path: "/etc/rhivos/certs/update-service.key".to_string(),
            storage_path: "/var/lib/containers/adapters".to_string(),
            data_broker_socket: "/run/kuksa/databroker.sock".to_string(),
            download_max_retries: 3,
            download_base_delay_ms: 1000,
            download_max_delay_ms: 30000,
            offload_threshold_hours: 24,
            offload_check_interval_minutes: 60,
            registry_username: std::env::var("REGISTRY_USERNAME").ok(),
            registry_password: std::env::var("REGISTRY_PASSWORD").ok(),
            token_refresh_buffer_secs: 60,
            log_level: "info".to_string(),
        }
    }
}

impl ServiceConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(addr) = std::env::var("UPDATE_SERVICE_LISTEN_ADDR") {
            config.listen_addr = addr;
        }

        if let Ok(path) = std::env::var("UPDATE_SERVICE_TLS_CERT") {
            config.tls_cert_path = path;
        }

        if let Ok(path) = std::env::var("UPDATE_SERVICE_TLS_KEY") {
            config.tls_key_path = path;
        }

        if let Ok(path) = std::env::var("UPDATE_SERVICE_STORAGE_PATH") {
            config.storage_path = path;
        }

        if let Ok(socket) = std::env::var("DATA_BROKER_SOCKET") {
            config.data_broker_socket = socket;
        }

        if let Ok(retries) = std::env::var("DOWNLOAD_MAX_RETRIES") {
            if let Ok(n) = retries.parse() {
                config.download_max_retries = n;
            }
        }

        if let Ok(delay) = std::env::var("DOWNLOAD_BASE_DELAY_MS") {
            if let Ok(n) = delay.parse() {
                config.download_base_delay_ms = n;
            }
        }

        if let Ok(hours) = std::env::var("OFFLOAD_THRESHOLD_HOURS") {
            if let Ok(n) = hours.parse() {
                config.offload_threshold_hours = n;
            }
        }

        if let Ok(level) = std::env::var("LOG_LEVEL") {
            config.log_level = level;
        }

        config
    }

    /// Check if registry credentials are configured.
    pub fn has_credentials(&self) -> bool {
        self.registry_username.is_some() && self.registry_password.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServiceConfig::default();

        assert_eq!(config.listen_addr, "0.0.0.0:50052");
        assert_eq!(config.download_max_retries, 3);
        assert_eq!(config.download_max_delay_ms, 30000);
        assert_eq!(config.offload_threshold_hours, 24);
    }

    #[test]
    fn test_has_credentials() {
        let mut config = ServiceConfig::default();

        // No credentials by default (unless env vars are set)
        config.registry_username = None;
        config.registry_password = None;
        assert!(!config.has_credentials());

        // With credentials
        config.registry_username = Some("user".to_string());
        config.registry_password = Some("pass".to_string());
        assert!(config.has_credentials());

        // Partial credentials
        config.registry_password = None;
        assert!(!config.has_credentials());
    }
}
