use serde::Deserialize;

/// Incoming lock/unlock command received from NATS.
///
/// The optional fields (`doors`, `source`, `vin`, `timestamp`) are part of the
/// protocol schema and are parsed from JSON.  They are not yet consumed by the
/// validation logic but are retained for future use.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct IncomingCommand {
    pub command_id: String,
    pub action: String,
    pub doors: Option<Vec<String>>,
    pub source: Option<String>,
    pub vin: Option<String>,
    pub timestamp: Option<i64>,
}

/// Error type for command validation.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    /// Bearer token is missing or does not match.
    ///
    /// Kept for completeness; the main command pipeline currently uses
    /// [`validate_bearer_token`] directly rather than returning this variant.
    #[allow(dead_code)]
    TokenRejected,
    /// Payload is not valid JSON.
    JsonParseFailed(String),
    /// JSON is valid but required fields are missing or have invalid values.
    ValidationFailed(String),
}

/// Validate a bearer token extracted from an Authorization header.
///
/// Expects the header value in the form "Bearer <token>". Returns true iff the
/// embedded token matches the configured expected token.
pub fn validate_bearer_token(header: Option<&str>, expected: &str) -> bool {
    match header {
        None => false,
        Some(value) => match value.strip_prefix("Bearer ") {
            Some(token) => token == expected,
            None => false,
        },
    }
}

/// Parse and validate a raw NATS command payload.
///
/// Returns `Ok(IncomingCommand)` when the payload is valid JSON with a non-empty
/// `command_id` and an `action` of `"lock"` or `"unlock"`. Returns `Err` otherwise.
pub fn parse_and_validate_command(payload: &[u8]) -> Result<IncomingCommand, CommandError> {
    let cmd: IncomingCommand = serde_json::from_slice(payload)
        .map_err(|e| CommandError::JsonParseFailed(e.to_string()))?;

    if cmd.command_id.is_empty() {
        return Err(CommandError::ValidationFailed(
            "command_id must be non-empty".to_string(),
        ));
    }

    if cmd.action != "lock" && cmd.action != "unlock" {
        return Err(CommandError::ValidationFailed(format!(
            "action must be 'lock' or 'unlock', got '{}'",
            cmd.action
        )));
    }

    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-04-3: Bearer token extraction — valid token
    #[test]
    fn test_bearer_token_valid() {
        let result = validate_bearer_token(Some("Bearer demo-token"), "demo-token");
        assert!(result, "matching token should be valid");
    }

    // TS-04-4: Bearer token validation — all three cases
    #[test]
    fn test_bearer_token_cases() {
        // Case 1: matching token
        assert!(
            validate_bearer_token(Some("Bearer demo-token"), "demo-token"),
            "matching bearer token should be valid"
        );
        // Case 2: wrong token
        assert!(
            !validate_bearer_token(Some("Bearer wrong-token"), "demo-token"),
            "non-matching bearer token should be invalid"
        );
        // Case 3: no header
        assert!(
            !validate_bearer_token(None, "demo-token"),
            "missing header should be invalid"
        );
    }

    // TS-04-5: Command JSON parsing — valid command
    #[test]
    fn test_command_parse_valid() {
        let input = br#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}"#;
        let result = parse_and_validate_command(input);
        assert!(result.is_ok(), "valid command should parse successfully");
        let cmd = result.unwrap();
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, "lock");
    }

    // TS-04-E3: Invalid bearer token → rejected
    #[test]
    fn test_bearer_token_invalid() {
        let result = validate_bearer_token(Some("Bearer wrong-token"), "demo-token");
        assert!(!result, "wrong bearer token should be rejected");
    }

    // TS-04-E4: Non-JSON payload → error
    #[test]
    fn test_command_invalid_json() {
        let result = parse_and_validate_command(b"not valid json {{{");
        assert!(result.is_err(), "invalid JSON should return error");
    }

    // TS-04-E5: Missing required field (command_id) → error
    #[test]
    fn test_command_missing_field() {
        let input = br#"{"action":"lock","doors":["driver"]}"#;
        let result = parse_and_validate_command(input);
        assert!(result.is_err(), "missing command_id should return error");
    }

    // Additional: missing action field → error
    #[test]
    fn test_command_missing_action() {
        let input = br#"{"command_id":"abc-123","doors":["driver"]}"#;
        let result = parse_and_validate_command(input);
        assert!(result.is_err(), "missing action should return error");
    }

    // Additional: invalid action value → error
    #[test]
    fn test_command_invalid_action() {
        let input = br#"{"command_id":"abc-123","action":"toggle","doors":["driver"]}"#;
        let result = parse_and_validate_command(input);
        assert!(result.is_err(), "invalid action value should return error");
    }

    // Additional: valid unlock command
    #[test]
    fn test_command_parse_unlock() {
        let input = br#"{"command_id":"def-456","action":"unlock"}"#;
        let result = parse_and_validate_command(input);
        assert!(result.is_ok(), "valid unlock command should parse successfully");
        let cmd = result.unwrap();
        assert_eq!(cmd.command_id, "def-456");
        assert_eq!(cmd.action, "unlock");
    }

    // Additional: empty command_id → error
    #[test]
    fn test_command_empty_command_id() {
        let input = br#"{"command_id":"","action":"lock"}"#;
        let result = parse_and_validate_command(input);
        assert!(result.is_err(), "empty command_id should return error");
    }
}
