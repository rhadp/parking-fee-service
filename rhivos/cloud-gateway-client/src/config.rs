#![allow(dead_code)]

use crate::errors::ConfigError;

/// Service configuration loaded from environment variables.
///
/// Only `VIN` is mandatory; all other fields have sensible defaults.
pub struct Config {
    pub vin: String,
    pub nats_url: String,
    pub databroker_addr: String,
    pub bearer_token: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Returns `Err(ConfigError::MissingVin)` when `VIN` is not set.
    /// All other variables fall back to documented defaults when absent.
    pub fn from_env() -> Result<Self, ConfigError> {
        todo!("implement in task group 2");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialises env-var access so parallel tests don't interfere.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// TS-04-1: Config reads VIN from environment; absent optional vars use defaults.
    ///
    /// Validates [04-REQ-1.1], [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4]
    #[test]
    fn test_config_defaults() {
        // Recover from a poisoned mutex (caused by a previous test panicking while
    // holding the lock) so that each test fails with the expected `todo!()` panic.
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("VIN", "TEST-VIN-001");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("DATABROKER_ADDR");
        std::env::remove_var("BEARER_TOKEN");

        let config = Config::from_env().expect("should succeed when VIN is set");
        assert_eq!(config.vin, "TEST-VIN-001");
        assert_eq!(config.nats_url, "nats://localhost:4222");
        assert_eq!(config.databroker_addr, "http://localhost:55556");
        assert_eq!(config.bearer_token, "demo-token");
    }

    /// TS-04-E1: Config returns MissingVin error when VIN env var is absent.
    ///
    /// Validates [04-REQ-1.E1]
    #[test]
    fn test_config_missing_vin() {
        // Recover from a poisoned mutex (caused by a previous test panicking while
    // holding the lock) so that each test fails with the expected `todo!()` panic.
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("VIN");

        let result = Config::from_env();
        assert!(
            matches!(result, Err(ConfigError::MissingVin)),
            "expected Err(MissingVin), got {:?}",
            result.err()
        );
    }

    /// TS-04-2: Config reads all custom environment variables correctly.
    ///
    /// Validates [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4]
    #[test]
    fn test_config_custom_values() {
        // Recover from a poisoned mutex (caused by a previous test panicking while
    // holding the lock) so that each test fails with the expected `todo!()` panic.
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("VIN", "MY-VIN");
        std::env::set_var("NATS_URL", "nats://custom:9222");
        std::env::set_var("DATABROKER_ADDR", "http://custom:55557");
        std::env::set_var("BEARER_TOKEN", "secret-token");

        let config = Config::from_env().expect("should succeed when VIN is set");
        assert_eq!(config.vin, "MY-VIN");
        assert_eq!(config.nats_url, "nats://custom:9222");
        assert_eq!(config.databroker_addr, "http://custom:55557");
        assert_eq!(config.bearer_token, "secret-token");
    }
}
