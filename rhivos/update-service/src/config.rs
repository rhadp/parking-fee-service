//! Configuration for the UPDATE_SERVICE.
//!
//! Loads configuration from environment variables with sensible defaults.
//! Implements 04-REQ-4.1 (configurable gRPC address) and
//! 04-REQ-6.1 (configurable inactivity timeout).

use std::env;
use std::time::Duration;

/// Configuration for the UPDATE_SERVICE loaded from environment variables.
///
/// | Variable              | Default                        | Description                                        |
/// |-----------------------|--------------------------------|----------------------------------------------------|
/// | `UPDATE_GRPC_ADDR`    | `0.0.0.0:50051`                | gRPC listen address                                |
/// | `REGISTRY_URL`        | `localhost:5000`               | OCI registry URL                                   |
/// | `OFFLOAD_TIMEOUT_HOURS` | `24`                         | Inactivity timeout before offloading (hours)       |
/// | `CONTAINER_STORE_PATH`| `/var/lib/containers/adapters/`| Container storage path                             |
#[derive(Debug, Clone)]
pub struct Config {
    /// gRPC listen address (default: 0.0.0.0:50051)
    pub grpc_addr: String,
    /// OCI registry URL (default: localhost:5000)
    pub registry_url: String,
    /// Inactivity timeout before offloading stopped adapters
    pub offload_timeout: Duration,
    /// Container storage path
    pub container_store_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grpc_addr: String::from("0.0.0.0:50051"),
            registry_url: String::from("localhost:5000"),
            offload_timeout: Duration::from_secs(24 * 3600), // 24 hours
            container_store_path: String::from("/var/lib/containers/adapters/"),
        }
    }
}

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let defaults = Config::default();

        let offload_timeout = env::var("OFFLOAD_TIMEOUT_HOURS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(|hours| Duration::from_secs(hours * 3600))
            .unwrap_or(defaults.offload_timeout);

        Self {
            grpc_addr: env::var("UPDATE_GRPC_ADDR").unwrap_or(defaults.grpc_addr),
            registry_url: env::var("REGISTRY_URL").unwrap_or(defaults.registry_url),
            offload_timeout,
            container_store_path: env::var("CONTAINER_STORE_PATH")
                .unwrap_or(defaults.container_store_path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TS-04-24: Configurable inactivity timeout
    // Requirement: 04-REQ-6.1
    // -----------------------------------------------------------------------

    #[test]
    fn test_configurable_offload_timeout_default() {
        // Default offload timeout should be 24 hours
        // Clear env var to test default
        std::env::remove_var("OFFLOAD_TIMEOUT_HOURS");
        let config = Config::from_env();
        assert_eq!(
            config.offload_timeout,
            Duration::from_secs(24 * 3600),
            "default offload timeout should be 24 hours"
        );
    }

    #[test]
    fn test_configurable_offload_timeout_custom() {
        // Custom offload timeout of 1 hour
        std::env::set_var("OFFLOAD_TIMEOUT_HOURS", "1");
        let config = Config::from_env();
        assert_eq!(
            config.offload_timeout,
            Duration::from_secs(3600),
            "custom offload timeout should be 1 hour"
        );
        // Cleanup
        std::env::remove_var("OFFLOAD_TIMEOUT_HOURS");
    }

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.grpc_addr, "0.0.0.0:50051");
        assert_eq!(config.registry_url, "localhost:5000");
        assert_eq!(config.offload_timeout, Duration::from_secs(24 * 3600));
        assert_eq!(
            config.container_store_path,
            "/var/lib/containers/adapters/"
        );
    }
}
