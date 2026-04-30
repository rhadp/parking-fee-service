use std::collections::HashMap;

use crate::errors::{AuthError, ValidationError};
use crate::models::CommandPayload;

/// Validate the bearer token from NATS message headers.
///
/// Extracts the `Authorization` header and checks it matches
/// `Bearer <expected_token>`.
pub fn validate_bearer_token(
    headers: &HashMap<String, String>,
    expected_token: &str,
) -> Result<(), AuthError> {
    let auth_value = headers
        .get("Authorization")
        .ok_or(AuthError::MissingHeader)?;

    let expected = format!("Bearer {expected_token}");
    if auth_value == &expected {
        Ok(())
    } else {
        Err(AuthError::InvalidToken)
    }
}

/// Validate the command payload structure.
///
/// Checks that the payload is valid JSON containing a non-empty `command_id`,
/// an `action` of `"lock"` or `"unlock"`, and a `doors` array.
/// Door values are NOT validated (that responsibility belongs to LOCKING_SERVICE).
pub fn validate_command_payload(payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    // First, parse as generic JSON to give precise error messages for
    // missing/invalid fields rather than serde's generic deserialization errors.
    let value: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| ValidationError::InvalidJson(e.to_string()))?;

    let obj = value
        .as_object()
        .ok_or_else(|| ValidationError::InvalidJson("expected JSON object".to_string()))?;

    // Validate command_id: must be present and a non-empty string.
    match obj.get("command_id") {
        None => return Err(ValidationError::MissingField("command_id".to_string())),
        Some(v) => match v.as_str() {
            Some("") => {
                return Err(ValidationError::MissingField("command_id".to_string()));
            }
            Some(_) => {}
            None => return Err(ValidationError::MissingField("command_id".to_string())),
        },
    }

    // Validate action: must be present and one of "lock" or "unlock".
    match obj.get("action") {
        None => return Err(ValidationError::MissingField("action".to_string())),
        Some(v) => match v.as_str() {
            Some("lock" | "unlock") => {}
            Some(other) => return Err(ValidationError::InvalidAction(other.to_string())),
            None => return Err(ValidationError::MissingField("action".to_string())),
        },
    }

    // Validate doors: must be present and an array.
    match obj.get("doors") {
        None => return Err(ValidationError::MissingField("doors".to_string())),
        Some(v) => {
            if !v.is_array() {
                return Err(ValidationError::MissingField("doors".to_string()));
            }
        }
    }

    // All validations passed -- deserialize into the typed struct.
    let cmd: CommandPayload = serde_json::from_slice(payload)
        .map_err(|e| ValidationError::InvalidJson(e.to_string()))?;

    Ok(cmd)
}
