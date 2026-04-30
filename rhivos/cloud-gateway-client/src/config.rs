use crate::errors::ConfigError;

const DEFAULT_NATS_URL: &str = "nats://localhost:4222";
const DEFAULT_DATABROKER_ADDR: &str = "http://localhost:55556";
const DEFAULT_BEARER_TOKEN: &str = "demo-token";

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
        let vin = std::env::var("VIN").map_err(|_| ConfigError::MissingVin)?;

        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
        let databroker_addr =
            std::env::var("DATABROKER_ADDR").unwrap_or_else(|_| DEFAULT_DATABROKER_ADDR.to_string());
        let bearer_token =
            std::env::var("BEARER_TOKEN").unwrap_or_else(|_| DEFAULT_BEARER_TOKEN.to_string());

        Ok(Config {
            vin,
            nats_url,
            databroker_addr,
            bearer_token,
        })
    }
}
