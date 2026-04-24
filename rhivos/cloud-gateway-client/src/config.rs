use crate::errors::ConfigError;

/// Service configuration read from environment variables.
#[derive(Debug)]
pub struct Config {
    pub vin: String,
    pub nats_url: String,
    pub databroker_addr: String,
    pub bearer_token: String,
}

impl Config {
    /// Reads configuration from environment variables.
    ///
    /// Returns `Err(ConfigError::MissingVin)` if the `VIN` environment
    /// variable is not set.
    pub fn from_env() -> Result<Config, ConfigError> {
        todo!()
    }
}
