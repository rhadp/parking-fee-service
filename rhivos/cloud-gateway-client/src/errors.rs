/// Errors for configuration loading.
#[derive(Debug, PartialEq)]
pub enum ConfigError {
    MissingVin,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingVin => write!(f, "VIN environment variable is not set"),
        }
    }
}

/// Errors for bearer token authentication.
#[derive(Debug, PartialEq)]
pub enum AuthError {
    MissingHeader,
    InvalidToken,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingHeader => write!(f, "Authorization header is missing"),
            AuthError::InvalidToken => write!(f, "Authorization token is invalid"),
        }
    }
}

/// Errors for command payload validation.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    InvalidJson(String),
    MissingField(String),
    InvalidAction(String),
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

/// Errors for NATS client operations.
#[derive(Debug)]
pub enum NatsError {
    ConnectionFailed(String),
    RetriesExhausted,
    PublishFailed(String),
    SubscribeFailed(String),
}

impl std::fmt::Display for NatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NatsError::ConnectionFailed(msg) => write!(f, "NATS connection failed: {msg}"),
            NatsError::RetriesExhausted => write!(f, "NATS connection retries exhausted"),
            NatsError::PublishFailed(msg) => write!(f, "NATS publish failed: {msg}"),
            NatsError::SubscribeFailed(msg) => write!(f, "NATS subscribe failed: {msg}"),
        }
    }
}

/// Errors for DATA_BROKER client operations.
#[derive(Debug)]
pub enum BrokerError {
    ConnectionFailed(String),
    WriteFailed(String),
    SubscribeFailed(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "DATA_BROKER connection failed: {msg}"),
            BrokerError::WriteFailed(msg) => write!(f, "DATA_BROKER write failed: {msg}"),
            BrokerError::SubscribeFailed(msg) => write!(f, "DATA_BROKER subscribe failed: {msg}"),
        }
    }
}
