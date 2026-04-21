use crate::errors::ConfigError;

/// Runtime configuration read from environment variables.
pub struct Config {
    /// Vehicle Identification Number — mandatory.
    pub vin: String,
    /// NATS server URL. Default: `nats://localhost:4222`.
    pub nats_url: String,
    /// DATA_BROKER gRPC address. Default: `http://localhost:55556`.
    pub databroker_addr: String,
    /// Bearer token used to authenticate incoming commands. Default: `demo-token`.
    pub bearer_token: String,
}

impl Config {
    /// Read and validate configuration from environment variables.
    ///
    /// Returns `Err(ConfigError::MissingVin)` when `VIN` is not set.
    pub fn from_env() -> Result<Self, ConfigError> {
        let vin = std::env::var("VIN").map_err(|_| ConfigError::MissingVin)?;
        let nats_url = std::env::var("NATS_URL")
            .unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let databroker_addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55556".to_string());
        let bearer_token = std::env::var("BEARER_TOKEN")
            .unwrap_or_else(|_| "demo-token".to_string());

        Ok(Config {
            vin,
            nats_url,
            databroker_addr,
            bearer_token,
        })
    }
}

// ─────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialises access to environment variables so tests do not interfere
    /// with each other when run in parallel.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // TS-04-1: Config reads VIN from environment
    // Validates: [04-REQ-1.1], defaults for [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4]
    #[test]
    fn ts_04_1_config_reads_vin_from_env() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("VIN", "TEST-VIN-001");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("DATABROKER_ADDR");
        std::env::remove_var("BEARER_TOKEN");

        let result = Config::from_env();

        // Clean up before asserting so a panic doesn't leave state dirty.
        std::env::remove_var("VIN");

        let config = result.expect("should return Ok(config) when VIN is set");
        assert_eq!(config.vin, "TEST-VIN-001");
        assert_eq!(config.nats_url, "nats://localhost:4222");
        assert_eq!(config.databroker_addr, "http://localhost:55556");
        assert_eq!(config.bearer_token, "demo-token");
    }

    // TS-04-E1: Config fails when VIN is missing
    // Validates: [04-REQ-1.E1]
    #[test]
    fn ts_04_e1_config_fails_when_vin_missing() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("VIN");

        let result = Config::from_env();

        assert!(
            matches!(result, Err(ConfigError::MissingVin)),
            "expected Err(ConfigError::MissingVin), got {:?}",
            result.map(|_| "<config>")
        );
    }

    // TS-04-2: Config reads all custom environment variables
    // Validates: [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4]
    #[test]
    fn ts_04_2_config_reads_custom_env_vars() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("VIN", "MY-VIN");
        std::env::set_var("NATS_URL", "nats://custom:9222");
        std::env::set_var("DATABROKER_ADDR", "http://custom:55557");
        std::env::set_var("BEARER_TOKEN", "secret-token");

        let result = Config::from_env();

        std::env::remove_var("VIN");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("DATABROKER_ADDR");
        std::env::remove_var("BEARER_TOKEN");

        let config = result.expect("should return Ok(config) when all vars are set");
        assert_eq!(config.nats_url, "nats://custom:9222");
        assert_eq!(config.databroker_addr, "http://custom:55557");
        assert_eq!(config.bearer_token, "secret-token");
    }
}
