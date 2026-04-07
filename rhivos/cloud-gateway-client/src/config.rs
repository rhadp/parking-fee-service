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
        // Stub: always returns a dummy config regardless of env vars
        Ok(Config {
            vin: String::new(),
            nats_url: String::new(),
            databroker_addr: String::new(),
            bearer_token: String::new(),
        })
    }
}
