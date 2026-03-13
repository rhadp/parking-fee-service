/// Trait abstracting NATS publish operations for testability.
#[allow(async_fn_in_trait)]
pub trait NatsPublisher {
    /// Publish a payload to a NATS subject.
    async fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), NatsError>;
}

/// Error type for NATS operations.
#[derive(Debug, Clone)]
pub struct NatsError(pub String);

impl std::fmt::Display for NatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NatsError: {}", self.0)
    }
}

impl std::error::Error for NatsError {}
