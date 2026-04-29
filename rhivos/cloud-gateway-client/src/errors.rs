/// Error returned when required configuration is missing.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// The `VIN` environment variable is not set.
    MissingVin,
}

/// Error returned when bearer token authentication fails.
#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    /// The `Authorization` header is missing from the NATS message.
    MissingHeader,
    /// The bearer token does not match the configured value.
    InvalidToken,
}

/// Error returned when command payload validation fails.
#[derive(Debug, PartialEq, Eq)]
pub enum ValidationError {
    /// The payload is not valid JSON.
    InvalidJson(String),
    /// A required field is missing or has an invalid type.
    MissingField(String),
    /// The `action` field has an unsupported value.
    InvalidAction(String),
}

/// Error from NATS client operations.
#[derive(Debug)]
pub enum NatsError {
    /// Failed to connect to the NATS server.
    ConnectionFailed(String),
    /// All retry attempts have been exhausted.
    RetriesExhausted,
    /// Failed to publish a message.
    PublishFailed(String),
    /// Failed to subscribe to a subject.
    SubscribeFailed(String),
}

/// Error from DATA_BROKER client operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Failed to connect to the DATA_BROKER.
    ConnectionFailed(String),
    /// Failed to write a signal value.
    WriteFailed(String),
    /// Failed to subscribe to a signal.
    SubscribeFailed(String),
}
