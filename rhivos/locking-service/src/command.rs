//! Command parsing and validation for lock/unlock payloads.
//!
//! `parse_command` deserializes a JSON string into a `LockCommand`.
//! `validate_command` enforces semantic constraints (non-empty command_id,
//! supported door values, etc.).

use serde::Deserialize;

/// The action requested by a command payload.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Lock,
    Unlock,
}

/// Deserialized representation of a lock/unlock command from DATA_BROKER.
#[derive(Debug, Clone, Deserialize)]
pub struct LockCommand {
    pub command_id: String,
    pub action: Action,
    pub doors: Vec<String>,
    // Optional metadata fields (03-REQ-2.4)
    pub source: Option<String>,
    pub vin: Option<String>,
    pub timestamp: Option<i64>,
}

/// Errors produced during command parsing or validation.
#[derive(Debug)]
pub enum CommandError {
    /// The payload is not syntactically valid JSON.
    InvalidJson(String),
    /// The payload is valid JSON but fails semantic validation
    /// (missing/empty required field, unrecognized action, etc.).
    InvalidCommand(String),
    /// The `doors` array contains a non-"driver" value.
    UnsupportedDoor(String),
}

impl CommandError {
    /// Returns the reason string used in command responses.
    pub fn reason(&self) -> &str {
        match self {
            CommandError::InvalidJson(_) => "invalid_json",
            CommandError::InvalidCommand(_) => "invalid_command",
            CommandError::UnsupportedDoor(_) => "unsupported_door",
        }
    }
}

/// Deserialize a JSON string into a `LockCommand`.
///
/// Returns `Err(CommandError::InvalidJson)` for malformed JSON.
/// Returns `Err(CommandError::InvalidCommand)` for missing/invalid required fields.
pub fn parse_command(json: &str) -> Result<LockCommand, CommandError> {
    serde_json::from_str(json).map_err(|e| {
        if e.is_data() {
            // Missing required field, unknown enum variant, wrong type, etc.
            CommandError::InvalidCommand(e.to_string())
        } else {
            // Syntax errors, unexpected EOF, IO errors → malformed JSON
            CommandError::InvalidJson(e.to_string())
        }
    })
}

/// Validate semantic constraints on a parsed `LockCommand`.
///
/// - `command_id` must be non-empty.
/// - `doors` must contain only "driver".
pub fn validate_command(cmd: &LockCommand) -> Result<(), CommandError> {
    if cmd.command_id.is_empty() {
        return Err(CommandError::InvalidCommand(
            "command_id must not be empty".to_owned(),
        ));
    }
    if cmd.doors.iter().any(|d| d != "driver") {
        return Err(CommandError::UnsupportedDoor(
            "doors array contains an unsupported value; only \"driver\" is allowed".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-2: Deserialise full lock command JSON
    #[test]
    fn test_parse_valid_command() {
        let json = r#"{
            "command_id": "abc-123",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "WDB123",
            "timestamp": 1700000000
        }"#;
        let cmd = parse_command(json).expect("should parse valid lock command");
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, Action::Lock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source.as_deref(), Some("companion_app"));
    }

    // TS-03-2: Deserialise unlock command
    #[test]
    fn test_parse_valid_unlock_command() {
        let json = r#"{
            "command_id": "def-456",
            "action": "unlock",
            "doors": ["driver"]
        }"#;
        let cmd = parse_command(json).expect("should parse valid unlock command");
        assert_eq!(cmd.command_id, "def-456");
        assert_eq!(cmd.action, Action::Unlock);
        assert!(cmd.source.is_none());
    }

    // TS-03-4: Empty command_id rejected
    #[test]
    fn test_validate_empty_command_id() {
        let json = r#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("JSON is valid, should parse");
        let result = validate_command(&cmd);
        assert!(result.is_err(), "empty command_id should be rejected");
        assert_eq!(
            result.unwrap_err().reason(),
            "invalid_command",
            "reason should be invalid_command"
        );
    }

    // TS-03-4: Missing command_id field rejected
    #[test]
    fn test_parse_missing_command_id() {
        let json = r#"{"action":"lock","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "missing command_id should be rejected");
    }

    // TS-03-5: Invalid action value rejected
    #[test]
    fn test_validate_invalid_action() {
        let json = r#"{"command_id":"x","action":"toggle","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(
            result.is_err(),
            "unknown action 'toggle' should produce a parse error"
        );
    }

    // TS-03-6: Non-"driver" door rejected with "unsupported_door"
    #[test]
    fn test_validate_unsupported_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["passenger"]}"#;
        let cmd = parse_command(json).expect("JSON is valid, should parse");
        let result = validate_command(&cmd);
        assert!(result.is_err(), "non-driver door should be rejected");
        assert_eq!(
            result.unwrap_err().reason(),
            "unsupported_door",
            "reason should be unsupported_door"
        );
    }

    // TS-03-E3: Non-JSON payload returns InvalidJson error
    #[test]
    fn test_parse_invalid_json() {
        let result = parse_command("not valid json {{{");
        assert!(result.is_err(), "invalid JSON should produce an error");
        assert!(
            matches!(result, Err(CommandError::InvalidJson(_))),
            "error should be CommandError::InvalidJson"
        );
    }

    // TS-03-E4: Missing action field rejected
    #[test]
    fn test_parse_missing_field() {
        let json = r#"{"command_id":"x","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "missing action field should produce error");
    }

    // TS-03-E5: "rear_left" door rejected with "unsupported_door"
    #[test]
    fn test_validate_non_driver_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["rear_left"]}"#;
        let cmd = parse_command(json).expect("JSON is valid, should parse");
        let result = validate_command(&cmd);
        assert!(result.is_err(), "non-driver door should be rejected");
        assert_eq!(
            result.unwrap_err().reason(),
            "unsupported_door",
            "reason should be unsupported_door"
        );
    }
}
