/// Configuration errors.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum ConfigError {
    MissingVin,
}

/// Authentication errors for bearer token validation.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum AuthError {
    MissingHeader,
    InvalidToken,
}

/// Command payload validation errors.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    InvalidJson(String),
    MissingField(String),
    InvalidAction(String),
}

/// NATS client errors.
#[allow(dead_code, clippy::enum_variant_names)]
#[derive(Debug)]
pub enum NatsError {
    ConnectionFailed(String),
    RetriesExhausted,
    PublishFailed(String),
    SubscribeFailed(String),
}

/// DATA_BROKER client errors.
#[allow(dead_code, clippy::enum_variant_names)]
#[derive(Debug)]
pub enum BrokerError {
    ConnectionFailed(String),
    WriteFailed(String),
    SubscribeFailed(String),
}
