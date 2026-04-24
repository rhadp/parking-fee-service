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
