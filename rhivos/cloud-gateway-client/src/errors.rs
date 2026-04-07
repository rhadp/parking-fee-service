//! Error types for the cloud-gateway-client service.

/// Configuration errors.
#[derive(Debug, PartialEq)]
pub enum ConfigError {
    /// The VIN environment variable is not set.
    MissingVin,
}

/// Authentication errors for bearer token validation.
#[derive(Debug, PartialEq)]
pub enum AuthError {
    /// The Authorization header is missing from the NATS message.
    MissingHeader,
    /// The bearer token does not match the configured value.
    InvalidToken,
}

/// Command payload validation errors.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    /// The payload is not valid JSON.
    InvalidJson(String),
    /// A required field is missing or has an invalid type.
    MissingField(String),
    /// The action field has an invalid value.
    InvalidAction(String),
}

/// NATS client errors.
#[derive(Debug)]
pub enum NatsError {
    /// Failed to connect to NATS server.
    ConnectionFailed(String),
    /// All retry attempts exhausted.
    RetriesExhausted,
    /// Failed to publish a message.
    PublishFailed(String),
    /// Failed to subscribe to a subject.
    SubscribeFailed(String),
}

/// DATA_BROKER client errors.
#[derive(Debug)]
pub enum BrokerError {
    /// Failed to connect to DATA_BROKER.
    ConnectionFailed(String),
    /// Failed to write a command signal.
    WriteFailed(String),
    /// Failed to subscribe to a signal.
    SubscribeFailed(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingVin => write!(f, "VIN environment variable is not set"),
        }
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingHeader => write!(f, "Authorization header is missing"),
            AuthError::InvalidToken => write!(f, "Invalid bearer token"),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidJson(msg) => write!(f, "Invalid JSON: {}", msg),
            ValidationError::MissingField(field) => write!(f, "Missing field: {}", field),
            ValidationError::InvalidAction(action) => write!(f, "Invalid action: {}", action),
        }
    }
}
