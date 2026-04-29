use std::collections::HashMap;

use crate::errors::{AuthError, ValidationError};
use crate::models::CommandPayload;

/// Validate the bearer token from NATS message headers.
///
/// Extracts the `Authorization` header and checks it matches
/// `Bearer <expected_token>`.
pub fn validate_bearer_token(
    _headers: &HashMap<String, String>,
    _expected_token: &str,
) -> Result<(), AuthError> {
    todo!()
}

/// Validate the command payload structure.
///
/// Checks that the payload is valid JSON containing a non-empty `command_id`,
/// an `action` of `"lock"` or `"unlock"`, and a `doors` array.
/// Door values are NOT validated (that responsibility belongs to LOCKING_SERVICE).
pub fn validate_command_payload(_payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    todo!()
}
