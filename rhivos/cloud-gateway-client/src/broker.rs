/// Trait abstracting the DATA_BROKER gRPC client for testability.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    /// Set a string-valued signal in DATA_BROKER.
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

/// Error type for broker operations.
#[derive(Debug, Clone)]
pub struct BrokerError(pub String);

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BrokerError: {}", self.0)
    }
}

impl std::error::Error for BrokerError {}
