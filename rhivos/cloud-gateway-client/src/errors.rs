/// Error returned when configuration is invalid.
#[derive(Debug, PartialEq)]
pub enum ConfigError {
    /// The VIN environment variable is not set.
    MissingVin,
}

/// Error returned when bearer token authentication fails.
#[derive(Debug, PartialEq)]
pub enum AuthError {
    /// The Authorization header is missing from the message.
    MissingHeader,
    /// The Authorization header is present but the token does not match.
    InvalidToken,
}

/// Error returned when command payload validation fails.
#[derive(Debug)]
pub enum ValidationError {
    /// The payload is not valid JSON.
    InvalidJson(String),
    /// A required field is missing or has an invalid type.
    MissingField(String),
    /// The action field is present but is not "lock" or "unlock".
    InvalidAction(String),
}

/// Error returned by NATS client operations.
#[derive(Debug)]
pub enum NatsError {
    /// The initial connection to the NATS server failed.
    ConnectionFailed(String),
    /// All retry attempts to connect to NATS have been exhausted.
    RetriesExhausted,
    /// A publish operation failed.
    PublishFailed(String),
    /// A subscribe operation failed.
    SubscribeFailed(String),
}

/// Error returned by DATA_BROKER client operations.
#[derive(Debug)]
pub enum BrokerError {
    /// The connection to DATA_BROKER could not be established.
    ConnectionFailed(String),
    /// A write (SetRequest) to DATA_BROKER failed.
    WriteFailed(String),
    /// A subscribe operation on DATA_BROKER failed.
    SubscribeFailed(String),
}
