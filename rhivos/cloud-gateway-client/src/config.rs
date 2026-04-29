use crate::errors::ConfigError;

/// Service configuration read from environment variables.
#[derive(Debug)]
pub struct Config {
    pub vin: String,
    /// NATS server URL. Default: `nats://localhost:4222`.
    pub nats_url: String,
    /// DATA_BROKER gRPC address. Default: `http://localhost:55556`.
    pub databroker_addr: String,
    /// Bearer token for command authentication. Default: `demo-token`.
    pub bearer_token: String,
}

impl Config {
    /// Read configuration from environment variables.
    ///
    /// Returns `Err(ConfigError::MissingVin)` if the `VIN` environment
    /// variable is not set.
    pub fn from_env() -> Result<Config, ConfigError> {
        todo!()
    }
}
