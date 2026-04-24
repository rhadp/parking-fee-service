use crate::errors::{AuthError, ValidationError};
use crate::models::CommandPayload;

/// Validates the bearer token from a NATS message Authorization header.
///
/// The `auth_header` parameter is the value of the `Authorization` header,
/// or `None` if the header is absent.
pub fn validate_bearer_token(auth_header: Option<&str>, expected: &str) -> Result<(), AuthError> {
    let header_value = auth_header.ok_or(AuthError::MissingHeader)?;

    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidToken)?;

    if token != expected {
        return Err(AuthError::InvalidToken);
    }

    Ok(())
}

/// Validates the structure of a command payload.
///
/// Returns the parsed `CommandPayload` if valid, or a `ValidationError`
/// describing why the payload is invalid.
///
/// Validation checks (in order):
/// 1. Payload must be valid JSON
/// 2. `command_id` must be present and a non-empty string
/// 3. `action` must be present and one of "lock" or "unlock"
/// 4. `doors` must be present and an array
///
/// Door values within the array are not validated; that responsibility
/// belongs to LOCKING_SERVICE ([04-REQ-6.4]).
pub fn validate_command_payload(payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    // Step 1: Parse as JSON Value for field-level validation
    let value: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| ValidationError::InvalidJson(e.to_string()))?;

    let obj = value
        .as_object()
        .ok_or_else(|| ValidationError::InvalidJson("payload is not a JSON object".to_string()))?;

    // Step 2: Validate command_id is present and non-empty
    match obj.get("command_id") {
        Some(serde_json::Value::String(id)) if !id.is_empty() => {}
        Some(serde_json::Value::String(_)) => {
            return Err(ValidationError::MissingField("command_id".to_string()));
        }
        _ => {
            return Err(ValidationError::MissingField("command_id".to_string()));
        }
    }

    // Step 3: Validate action is present and is "lock" or "unlock"
    match obj.get("action") {
        Some(serde_json::Value::String(action)) => {
            if action != "lock" && action != "unlock" {
                return Err(ValidationError::InvalidAction(action.clone()));
            }
        }
        _ => {
            return Err(ValidationError::MissingField("action".to_string()));
        }
    }

    // Step 4: Validate doors is present and is an array
    match obj.get("doors") {
        Some(serde_json::Value::Array(_)) => {}
        _ => {
            return Err(ValidationError::MissingField("doors".to_string()));
        }
    }

    // All checks passed — deserialize into CommandPayload
    serde_json::from_value(value)
        .map_err(|e| ValidationError::InvalidJson(e.to_string()))
}
