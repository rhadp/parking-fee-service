/// Command validation: bearer token authentication and payload structural validation.
use crate::errors::{AuthError, ValidationError};
use crate::models::CommandPayload;

/// Validate bearer token from NATS message headers.
///
/// Checks that the `Authorization` header is present and matches
/// `Bearer <expected_token>`.
pub fn validate_bearer_token(
    auth_header: Option<&str>,
    expected_token: &str,
) -> Result<(), AuthError> {
    // Stub: always succeeds regardless of input
    let _ = (auth_header, expected_token);
    Ok(())
}

/// Validate command payload structure.
///
/// Checks that the payload is valid JSON with required fields:
/// - `command_id`: non-empty string
/// - `action`: one of "lock" or "unlock"
/// - `doors`: array
pub fn validate_command_payload(payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    // Stub: always returns a dummy payload regardless of input
    let _ = payload;
    Ok(CommandPayload {
        command_id: String::new(),
        action: String::new(),
        doors: Vec::new(),
        extra: serde_json::Map::new(),
    })
}
