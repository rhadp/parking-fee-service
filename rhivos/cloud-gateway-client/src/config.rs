//! Service configuration from environment variables.

use crate::errors::ConfigError;

/// Service configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub vin: String,
    pub nats_url: String,
    pub databroker_addr: String,
    pub bearer_token: String,
}

impl Config {
    /// Read configuration from environment variables.
    ///
    /// Returns `ConfigError::MissingVin` if the `VIN` variable is not set.
    /// Other variables use defaults if not set.
    pub fn from_env() -> Result<Config, ConfigError> {
        todo!("Config::from_env not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // TS-04-1: Config reads VIN from environment
    #[test]
    fn test_config_reads_vin() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("VIN", "TEST-VIN-001");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("DATABROKER_ADDR");
        std::env::remove_var("BEARER_TOKEN");

        let config = Config::from_env().expect("should parse config");
        assert_eq!(config.vin, "TEST-VIN-001");
        assert_eq!(config.nats_url, "nats://localhost:4222");
        assert_eq!(config.databroker_addr, "http://localhost:55556");
        assert_eq!(config.bearer_token, "demo-token");

        std::env::remove_var("VIN");
    }

    // TS-04-E1: Config fails when VIN is missing
    #[test]
    fn test_config_missing_vin() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("VIN");

        let result = Config::from_env();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConfigError::MissingVin);
    }

    // TS-04-2: Config reads all custom environment variables
    #[test]
    fn test_config_custom_env_vars() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("VIN", "MY-VIN");
        std::env::set_var("NATS_URL", "nats://custom:9222");
        std::env::set_var("DATABROKER_ADDR", "http://custom:55557");
        std::env::set_var("BEARER_TOKEN", "secret-token");

        let config = Config::from_env().expect("should parse config");
        assert_eq!(config.nats_url, "nats://custom:9222");
        assert_eq!(config.databroker_addr, "http://custom:55557");
        assert_eq!(config.bearer_token, "secret-token");

        std::env::remove_var("VIN");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("DATABROKER_ADDR");
        std::env::remove_var("BEARER_TOKEN");
    }
}
