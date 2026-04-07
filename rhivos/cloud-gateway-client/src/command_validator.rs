//! Command validation: bearer token authentication and payload structural validation.
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
    let header = auth_header.ok_or(AuthError::MissingHeader)?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidToken)?;
    if token == expected_token {
        Ok(())
    } else {
        Err(AuthError::InvalidToken)
    }
}

/// Validate command payload structure.
///
/// Checks that the payload is valid JSON with required fields:
/// - `command_id`: non-empty string
/// - `action`: one of "lock" or "unlock"
/// - `doors`: array (values are not validated)
pub fn validate_command_payload(payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    let value: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| ValidationError::InvalidJson(e.to_string()))?;

    let obj = value
        .as_object()
        .ok_or_else(|| ValidationError::InvalidJson("expected JSON object".to_string()))?;

    // Validate command_id
    let command_id = obj
        .get("command_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidationError::MissingField("command_id".to_string()))?;
    if command_id.is_empty() {
        return Err(ValidationError::MissingField("command_id".to_string()));
    }

    // Validate action
    let action = obj
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidationError::MissingField("action".to_string()))?;
    if action != "lock" && action != "unlock" {
        return Err(ValidationError::InvalidAction(action.to_string()));
    }

    // Validate doors (must be present; values are not validated)
    let doors_val = obj
        .get("doors")
        .ok_or_else(|| ValidationError::MissingField("doors".to_string()))?;
    let doors_arr = doors_val
        .as_array()
        .ok_or_else(|| ValidationError::MissingField("doors".to_string()))?;
    let doors: Vec<String> = doors_arr
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    // Collect any extra fields
    let extra: serde_json::Map<String, serde_json::Value> = obj
        .iter()
        .filter(|(k, _)| k.as_str() != "command_id" && k.as_str() != "action" && k.as_str() != "doors")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    Ok(CommandPayload {
        command_id: command_id.to_string(),
        action: action.to_string(),
        doors,
        extra,
    })
}
