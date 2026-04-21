/// Configuration error.
#[derive(Debug, PartialEq)]
pub enum ConfigError {
    MissingVin,
}

/// Authentication error for bearer-token validation.
#[derive(Debug, PartialEq)]
pub enum AuthError {
    MissingHeader,
    InvalidToken,
}

/// Command payload validation error.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    InvalidJson(String),
    MissingField(String),
    InvalidAction(String),
}

/// NATS client error.
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
            NatsError::ConnectionFailed(msg) => write!(f, "NATS connection failed: {}", msg),
            NatsError::RetriesExhausted => write!(f, "NATS connection retries exhausted"),
            NatsError::PublishFailed(msg) => write!(f, "NATS publish failed: {}", msg),
            NatsError::SubscribeFailed(msg) => write!(f, "NATS subscribe failed: {}", msg),
        }
    }
}

/// DATA_BROKER gRPC client error.
#[derive(Debug)]
pub enum BrokerError {
    ConnectionFailed(String),
    WriteFailed(String),
    SubscribeFailed(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "Broker connection failed: {}", msg),
            BrokerError::WriteFailed(msg) => write!(f, "Broker write failed: {}", msg),
            BrokerError::SubscribeFailed(msg) => write!(f, "Broker subscribe failed: {}", msg),
        }
    }
}
