//! Service configuration for LOCKING_SERVICE.
//!
//! Configuration can be loaded from environment variables or use defaults
//! as specified in the design document.

use std::time::Duration;

/// Service configuration for the LOCKING_SERVICE.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// UDS socket path for gRPC server.
    pub socket_path: String,
    /// DATA_BROKER UDS socket path.
    pub data_broker_socket: String,
    /// Command execution timeout.
    pub execution_timeout: Duration,
    /// Safety validation timeout.
    pub validation_timeout: Duration,
    /// Max retries for DATA_BROKER publish.
    pub publish_max_retries: u32,
    /// Base delay for exponential backoff.
    pub publish_base_delay: Duration,
    /// Valid auth tokens (demo-grade, not production).
    pub valid_tokens: Vec<String>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            socket_path: "/run/rhivos/locking.sock".to_string(),
            data_broker_socket: "/run/kuksa/databroker.sock".to_string(),
            execution_timeout: Duration::from_millis(500),
            validation_timeout: Duration::from_millis(100),
            publish_max_retries: 3,
            publish_base_delay: Duration::from_millis(50),
            valid_tokens: vec!["demo-token".to_string()],
        }
    }
}

impl ServiceConfig {
    /// Creates a new ServiceConfig with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads configuration from environment variables.
    ///
    /// Supported environment variables:
    /// - `LOCKING_SOCKET_PATH`: UDS socket path for gRPC server
    /// - `DATA_BROKER_SOCKET`: DATA_BROKER UDS socket path
    /// - `EXECUTION_TIMEOUT_MS`: Command execution timeout in milliseconds
    /// - `VALIDATION_TIMEOUT_MS`: Safety validation timeout in milliseconds
    /// - `PUBLISH_MAX_RETRIES`: Max retries for DATA_BROKER publish
    /// - `PUBLISH_BASE_DELAY_MS`: Base delay for exponential backoff in milliseconds
    /// - `VALID_TOKENS`: Comma-separated list of valid auth tokens
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("LOCKING_SOCKET_PATH") {
            config.socket_path = val;
        }

        if let Ok(val) = std::env::var("DATA_BROKER_SOCKET") {
            config.data_broker_socket = val;
        }

        if let Ok(val) = std::env::var("EXECUTION_TIMEOUT_MS") {
            if let Ok(ms) = val.parse::<u64>() {
                config.execution_timeout = Duration::from_millis(ms);
            }
        }

        if let Ok(val) = std::env::var("VALIDATION_TIMEOUT_MS") {
            if let Ok(ms) = val.parse::<u64>() {
                config.validation_timeout = Duration::from_millis(ms);
            }
        }

        if let Ok(val) = std::env::var("PUBLISH_MAX_RETRIES") {
            if let Ok(retries) = val.parse::<u32>() {
                config.publish_max_retries = retries;
            }
        }

        if let Ok(val) = std::env::var("PUBLISH_BASE_DELAY_MS") {
            if let Ok(ms) = val.parse::<u64>() {
                config.publish_base_delay = Duration::from_millis(ms);
            }
        }

        if let Ok(val) = std::env::var("VALID_TOKENS") {
            let tokens: Vec<String> = val.split(',').map(|s| s.trim().to_string()).collect();
            if !tokens.is_empty() {
                config.valid_tokens = tokens;
            }
        }

        config
    }

    /// Builder method to set the socket path.
    pub fn with_socket_path(mut self, path: impl Into<String>) -> Self {
        self.socket_path = path.into();
        self
    }

    /// Builder method to set the data broker socket.
    pub fn with_data_broker_socket(mut self, path: impl Into<String>) -> Self {
        self.data_broker_socket = path.into();
        self
    }

    /// Builder method to set the execution timeout.
    pub fn with_execution_timeout(mut self, timeout: Duration) -> Self {
        self.execution_timeout = timeout;
        self
    }

    /// Builder method to set the validation timeout.
    pub fn with_validation_timeout(mut self, timeout: Duration) -> Self {
        self.validation_timeout = timeout;
        self
    }

    /// Builder method to set the publish max retries.
    pub fn with_publish_max_retries(mut self, retries: u32) -> Self {
        self.publish_max_retries = retries;
        self
    }

    /// Builder method to set the publish base delay.
    pub fn with_publish_base_delay(mut self, delay: Duration) -> Self {
        self.publish_base_delay = delay;
        self
    }

    /// Builder method to set the valid tokens.
    pub fn with_valid_tokens(mut self, tokens: Vec<String>) -> Self {
        self.valid_tokens = tokens;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServiceConfig::default();
        assert_eq!(config.socket_path, "/run/rhivos/locking.sock");
        assert_eq!(config.data_broker_socket, "/run/kuksa/databroker.sock");
        assert_eq!(config.execution_timeout, Duration::from_millis(500));
        assert_eq!(config.validation_timeout, Duration::from_millis(100));
        assert_eq!(config.publish_max_retries, 3);
        assert_eq!(config.publish_base_delay, Duration::from_millis(50));
        assert_eq!(config.valid_tokens, vec!["demo-token".to_string()]);
    }

    #[test]
    fn test_builder_methods() {
        let config = ServiceConfig::new()
            .with_socket_path("/custom/socket.sock")
            .with_execution_timeout(Duration::from_secs(1))
            .with_publish_max_retries(5);

        assert_eq!(config.socket_path, "/custom/socket.sock");
        assert_eq!(config.execution_timeout, Duration::from_secs(1));
        assert_eq!(config.publish_max_retries, 5);
    }
}
