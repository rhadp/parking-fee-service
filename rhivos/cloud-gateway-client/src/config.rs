use crate::errors::ConfigError;

/// Service configuration, loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Vehicle Identification Number — used in all NATS subject paths.
    pub vin: String,
    /// NATS server URL. Default: `nats://localhost:4222`.
    pub nats_url: String,
    /// DATA_BROKER gRPC address. Default: `http://localhost:55556`.
    pub databroker_addr: String,
    /// Bearer token for command authentication. Default: `demo-token`.
    pub bearer_token: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Returns `Err(ConfigError::MissingVin)` if `VIN` is not set.
    pub fn from_env() -> Result<Self, ConfigError> {
        let vin = std::env::var("VIN").map_err(|_| ConfigError::MissingVin)?;
        let nats_url = std::env::var("NATS_URL")
            .unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let databroker_addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55556".to_string());
        let bearer_token = std::env::var("BEARER_TOKEN")
            .unwrap_or_else(|_| "demo-token".to_string());
        Ok(Config { vin, nats_url, databroker_addr, bearer_token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Serialize env-var tests: all tests that touch process-global environment
    // variables must hold this lock for the duration of the test to prevent
    // race conditions when the test suite runs in parallel.
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // TS-04-E1: Config fails when VIN is missing
    #[test]
    fn ts_04_e1_config_fails_when_vin_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("VIN");
        let result = Config::from_env();
        assert_eq!(
            result.unwrap_err(),
            ConfigError::MissingVin,
            "expected MissingVin error when VIN env var is absent"
        );
    }

    // TS-04-1: Config reads VIN from environment and uses defaults
    #[test]
    fn ts_04_1_config_reads_vin_from_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("VIN", "TEST-VIN-001");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("DATABROKER_ADDR");
        std::env::remove_var("BEARER_TOKEN");

        let config = Config::from_env().expect("expected Ok config");
        assert_eq!(config.vin, "TEST-VIN-001");
        assert_eq!(config.nats_url, "nats://localhost:4222");
        assert_eq!(config.databroker_addr, "http://localhost:55556");
        assert_eq!(config.bearer_token, "demo-token");

        std::env::remove_var("VIN");
    }

    // TS-04-2: Config reads all custom environment variables
    #[test]
    fn ts_04_2_config_reads_all_custom_env_vars() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("VIN", "MY-VIN");
        std::env::set_var("NATS_URL", "nats://custom:9222");
        std::env::set_var("DATABROKER_ADDR", "http://custom:55557");
        std::env::set_var("BEARER_TOKEN", "secret-token");

        let config = Config::from_env().expect("expected Ok config");
        assert_eq!(config.vin, "MY-VIN");
        assert_eq!(config.nats_url, "nats://custom:9222");
        assert_eq!(config.databroker_addr, "http://custom:55557");
        assert_eq!(config.bearer_token, "secret-token");

        std::env::remove_var("VIN");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("DATABROKER_ADDR");
        std::env::remove_var("BEARER_TOKEN");
    }
}
