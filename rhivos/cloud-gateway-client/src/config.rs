/// Configuration for the CLOUD_GATEWAY_CLIENT.
#[derive(Debug, Clone)]
pub struct Config {
    pub vin: String,
    pub nats_url: String,
    pub databroker_addr: String,
    pub bearer_token: String,
}

/// Errors that can occur when loading configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    /// The required VIN environment variable is not set.
    MissingVin,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingVin => write!(f, "VIN environment variable is required but not set"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Load configuration from environment variables.
///
/// Required: VIN
/// Optional: NATS_URL (default: nats://localhost:4222), DATABROKER_ADDR (default: http://localhost:55556),
///           BEARER_TOKEN (default: demo-token)
pub fn load_config() -> Result<Config, ConfigError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    // TS-04-1: NATS_URL default
    #[test]
    fn test_nats_url_default() {
        std::env::remove_var("NATS_URL");
        std::env::set_var("VIN", "TEST_VIN_001");
        let config = load_config().expect("config should load with VIN set");
        assert_eq!(config.nats_url, "nats://localhost:4222");
        std::env::remove_var("VIN");
    }

    // TS-04-1: NATS_URL from environment
    #[test]
    fn test_nats_url_env() {
        std::env::set_var("VIN", "TEST_VIN_001");
        std::env::set_var("NATS_URL", "nats://10.0.0.5:4222");
        let config = load_config().expect("config should load");
        assert_eq!(config.nats_url, "nats://10.0.0.5:4222");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("VIN");
    }

    // TS-04-12: DATABROKER_ADDR default
    #[test]
    fn test_databroker_addr_default() {
        std::env::remove_var("DATABROKER_ADDR");
        std::env::set_var("VIN", "TEST_VIN_001");
        let config = load_config().expect("config should load");
        assert_eq!(config.databroker_addr, "http://localhost:55556");
        std::env::remove_var("VIN");
    }

    // TS-04-13: VIN from environment
    #[test]
    fn test_vin_from_env() {
        std::env::set_var("VIN", "WDB123456789");
        let config = load_config().expect("config should load when VIN is set");
        assert_eq!(config.vin, "WDB123456789");
        std::env::remove_var("VIN");
    }

    // TS-04-E10: VIN not set → error
    #[test]
    fn test_vin_missing() {
        std::env::remove_var("VIN");
        let result = load_config();
        assert!(result.is_err(), "load_config should fail when VIN is not set");
        assert_eq!(result.unwrap_err(), ConfigError::MissingVin);
    }
}
