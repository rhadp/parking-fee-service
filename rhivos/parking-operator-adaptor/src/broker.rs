use std::fmt;

/// Error type for DATA_BROKER operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection to DATA_BROKER failed.
    ConnectionFailed(String),
    /// A DATA_BROKER operation failed.
    OperationFailed(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "connection failed: {msg}"),
            BrokerError::OperationFailed(msg) => write!(f, "operation failed: {msg}"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// Trait abstracting DATA_BROKER gRPC client operations.
#[allow(async_fn_in_trait)]
pub trait DataBrokerClient {
    /// Set a boolean signal value in DATA_BROKER.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
}
