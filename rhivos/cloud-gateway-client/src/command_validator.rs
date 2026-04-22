//! Command authentication and payload validation.
//!
//! Pure functions with no I/O dependencies. Validates bearer tokens
//! from NATS message headers and command payload structure.

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
    todo!("validate_bearer_token not yet implemented")
}

/// Validate a command payload.
///
/// Uses two-phase parsing:
/// 1. Parse as JSON — syntax errors produce `InvalidJson`
/// 2. Validate required fields — missing/invalid fields produce
///    `MissingField` or `InvalidAction`
///
/// The `doors` array contents are not validated; that responsibility
/// belongs to LOCKING_SERVICE.
pub fn validate_command_payload(_payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    todo!("validate_command_payload not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-04-3: Bearer token validation accepts valid token
    #[test]
    fn test_bearer_token_valid() {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer demo-token".to_string(),
        );
        let result = validate_bearer_token(&headers, "demo-token");
        assert!(result.is_ok());
    }

    // TS-04-E2: Bearer token validation rejects missing header
    #[test]
    fn test_bearer_token_missing_header() {
        let headers = HashMap::new();
        let result = validate_bearer_token(&headers, "demo-token");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), AuthError::MissingHeader);
    }

    // TS-04-E3: Bearer token validation rejects wrong token
    #[test]
    fn test_bearer_token_wrong_token() {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer wrong-token".to_string(),
        );
        let result = validate_bearer_token(&headers, "demo-token");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), AuthError::InvalidToken);
    }

    // TS-04-E4: Bearer token validation rejects malformed header
    #[test]
    fn test_bearer_token_malformed_header() {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "NotBearer demo-token".to_string(),
        );
        let result = validate_bearer_token(&headers, "demo-token");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), AuthError::InvalidToken);
    }

    // TS-04-4: Command validation accepts valid payload
    #[test]
    fn test_command_valid_payload() {
        let payload = r#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, "lock");
        assert_eq!(cmd.doors, vec!["driver"]);
    }

    // TS-04-5: Command validation accepts unlock action
    #[test]
    fn test_command_unlock_action() {
        let payload = r#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.action, "unlock");
    }

    // TS-04-E5: Command validation rejects invalid JSON
    #[test]
    fn test_command_invalid_json() {
        let payload = b"not-valid-json{{";
        let result = validate_command_payload(payload);
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ValidationError::InvalidJson(_)),
            "expected InvalidJson error"
        );
    }

    // TS-04-E6: Command validation rejects missing command_id
    #[test]
    fn test_command_missing_command_id() {
        let payload = r#"{"action":"lock","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("command_id".to_string())
        );
    }

    // TS-04-E7: Command validation rejects empty command_id
    #[test]
    fn test_command_empty_command_id() {
        let payload = r#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("command_id".to_string())
        );
    }

    // TS-04-E8: Command validation rejects missing action
    #[test]
    fn test_command_missing_action() {
        let payload = r#"{"command_id":"abc","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("action".to_string())
        );
    }

    // TS-04-E9: Command validation rejects invalid action
    #[test]
    fn test_command_invalid_action() {
        let payload = r#"{"command_id":"abc","action":"open","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ValidationError::InvalidAction("open".to_string())
        );
    }

    // TS-04-E10: Command validation rejects missing doors
    #[test]
    fn test_command_missing_doors() {
        let payload = r#"{"command_id":"abc","action":"lock"}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("doors".to_string())
        );
    }

    // TS-04-6: Command validation does not validate door values
    #[test]
    fn test_command_accepts_any_door_values() {
        let payload = r#"{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.doors, vec!["unknown-door", "another"]);
    }
}
