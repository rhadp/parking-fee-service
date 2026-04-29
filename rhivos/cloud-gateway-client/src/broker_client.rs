use crate::config::Config;
use crate::errors::BrokerError;

/// Client for DATA_BROKER gRPC operations.
pub struct BrokerClient {
    _private: (),
}

impl BrokerClient {
    /// Connect to the DATA_BROKER at the configured address.
    ///
    /// Returns `Err(BrokerError::ConnectionFailed)` if the connection
    /// cannot be established.
    pub async fn connect(_config: &Config) -> Result<Self, BrokerError> {
        todo!()
    }
}
