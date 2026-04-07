/// Error type for broker operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection failed.
    ConnectionFailed(String),
    /// Operation failed.
    OperationFailed(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "connection failed: {}", msg),
            BrokerError::OperationFailed(msg) => write!(f, "operation failed: {}", msg),
        }
    }
}

impl std::error::Error for BrokerError {}

/// Trait abstracting DATA_BROKER gRPC client operations.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    /// Read a float signal value.
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    /// Read a boolean signal value.
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    /// Write a boolean signal value.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    /// Write a string signal value.
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}
