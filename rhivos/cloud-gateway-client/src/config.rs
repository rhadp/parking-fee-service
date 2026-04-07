/// Configuration module for reading and validating environment variables.
use crate::errors::ConfigError;

/// Service configuration parsed from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub vin: String,
    pub nats_url: String,
    pub databroker_addr: String,
    pub bearer_token: String,
}

impl Config {
    /// Read configuration from environment variables.
    ///
    /// Returns `Err(ConfigError::MissingVin)` if the `VIN` env var is not set.
    /// Applies defaults for optional variables.
    pub fn from_env() -> Result<Config, ConfigError> {
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
