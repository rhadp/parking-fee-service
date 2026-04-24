use crate::errors::{AuthError, ValidationError};
use crate::models::CommandPayload;

/// Validates the bearer token from a NATS message Authorization header.
///
/// The `auth_header` parameter is the value of the `Authorization` header,
/// or `None` if the header is absent.
pub fn validate_bearer_token(auth_header: Option<&str>, expected: &str) -> Result<(), AuthError> {
    let _ = (auth_header, expected);
    todo!()
}

/// Validates the structure of a command payload.
///
/// Returns the parsed `CommandPayload` if valid, or a `ValidationError`
/// describing why the payload is invalid.
pub fn validate_command_payload(payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    let _ = payload;
    todo!()
}
