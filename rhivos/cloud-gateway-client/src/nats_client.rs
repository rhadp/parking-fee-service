use crate::config::Config;
use crate::errors::NatsError;

/// Client for NATS messaging operations.
pub struct NatsClient {
    _private: (),
}

impl NatsClient {
    /// Connect to the NATS server with exponential backoff retry.
    ///
    /// Retries up to 5 attempts with delays of 1s, 2s, 4s, 8s.
    /// Returns `Err(NatsError::RetriesExhausted)` if all attempts fail.
    pub async fn connect(_config: &Config) -> Result<Self, NatsError> {
        todo!()
    }
}
