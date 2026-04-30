use std::fmt;

/// Error returned when required configuration is missing.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// The `VIN` environment variable is not set.
    MissingVin,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingVin => write!(f, "required environment variable VIN is not set"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Error returned when bearer token authentication fails.
#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    /// The `Authorization` header is missing from the NATS message.
    MissingHeader,
    /// The bearer token does not match the configured value.
    InvalidToken,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::MissingHeader => write!(f, "missing Authorization header"),
            AuthError::InvalidToken => write!(f, "invalid bearer token"),
        }
    }
}

impl std::error::Error for AuthError {}

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

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidJson(msg) => write!(f, "invalid JSON: {msg}"),
            ValidationError::MissingField(field) => write!(f, "missing required field: {field}"),
            ValidationError::InvalidAction(action) => write!(f, "invalid action: {action}"),
        }
    }
}

impl std::error::Error for ValidationError {}

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

impl fmt::Display for NatsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NatsError::ConnectionFailed(msg) => write!(f, "NATS connection failed: {msg}"),
            NatsError::RetriesExhausted => write!(f, "NATS connection retries exhausted"),
            NatsError::PublishFailed(msg) => write!(f, "NATS publish failed: {msg}"),
            NatsError::SubscribeFailed(msg) => write!(f, "NATS subscribe failed: {msg}"),
        }
    }
}

impl std::error::Error for NatsError {}

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

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => {
                write!(f, "DATA_BROKER connection failed: {msg}")
            }
            BrokerError::WriteFailed(msg) => write!(f, "DATA_BROKER write failed: {msg}"),
            BrokerError::SubscribeFailed(msg) => {
                write!(f, "DATA_BROKER subscribe failed: {msg}")
            }
        }
    }
}

impl std::error::Error for BrokerError {}
