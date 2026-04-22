//! Error types for the cloud-gateway-client service.

/// Configuration errors.
#[derive(Debug, PartialEq)]
pub enum ConfigError {
    /// VIN environment variable is not set.
    MissingVin,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingVin => write!(f, "VIN environment variable is required"),
        }
    }
}

/// Authentication errors.
#[derive(Debug, PartialEq)]
pub enum AuthError {
    /// Authorization header is missing.
    MissingHeader,
    /// Bearer token does not match.
    InvalidToken,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingHeader => write!(f, "Authorization header is missing"),
            AuthError::InvalidToken => write!(f, "invalid bearer token"),
        }
    }
}

/// Command validation errors.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    /// Payload is not valid JSON.
    InvalidJson(String),
    /// A required field is missing or has an invalid type.
    MissingField(String),
    /// Action field is not "lock" or "unlock".
    InvalidAction(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidJson(e) => write!(f, "invalid JSON: {e}"),
            ValidationError::MissingField(field) => write!(f, "missing field: {field}"),
            ValidationError::InvalidAction(action) => write!(f, "invalid action: {action}"),
        }
    }
}

/// NATS communication errors.
#[derive(Debug, PartialEq)]
pub enum NatsError {
    /// NATS connection failed.
    ConnectionFailed(String),
    /// All retry attempts exhausted.
    RetriesExhausted,
    /// Failed to publish a message.
    PublishFailed(String),
    /// Failed to subscribe to a subject.
    SubscribeFailed(String),
}

impl std::fmt::Display for NatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NatsError::ConnectionFailed(e) => write!(f, "NATS connection failed: {e}"),
            NatsError::RetriesExhausted => write!(f, "NATS connection retries exhausted"),
            NatsError::PublishFailed(e) => write!(f, "NATS publish failed: {e}"),
            NatsError::SubscribeFailed(e) => write!(f, "NATS subscribe failed: {e}"),
        }
    }
}

/// DATA_BROKER communication errors.
#[derive(Debug, PartialEq)]
pub enum BrokerError {
    /// Failed to connect to DATA_BROKER.
    ConnectionFailed(String),
    /// Failed to write a signal.
    WriteFailed(String),
    /// Failed to subscribe to a signal.
    SubscribeFailed(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::ConnectionFailed(e) => write!(f, "broker connection failed: {e}"),
            BrokerError::WriteFailed(e) => write!(f, "broker write failed: {e}"),
            BrokerError::SubscribeFailed(e) => write!(f, "broker subscribe failed: {e}"),
        }
    }
}
