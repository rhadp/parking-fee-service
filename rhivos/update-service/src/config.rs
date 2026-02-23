//! Configuration for the UPDATE_SERVICE.
//!
//! Stub module — implementation will be added in task group 5.

use std::time::Duration;

/// Configuration for the UPDATE_SERVICE loaded from environment variables.
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

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        // Stub: returns hardcoded defaults — real implementation in task group 5
        Self {
            grpc_addr: String::from("0.0.0.0:50051"),
            registry_url: String::from("localhost:5000"),
            offload_timeout: Duration::from_secs(0), // Stub: wrong default
            container_store_path: String::from("/var/lib/containers/adapters/"),
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
}
