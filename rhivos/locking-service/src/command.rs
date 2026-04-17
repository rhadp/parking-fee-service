//! Command parsing and validation.
//!
//! Deserializes JSON lock/unlock commands from `Vehicle.Command.Door.Lock`
//! and validates all required fields before processing.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ── Data models ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Lock,
    Unlock,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LockCommand {
    pub command_id: String,
    pub action: Action,
    pub doors: Vec<String>,
    // Optional metadata — must not affect processing logic (03-REQ-2.4).
    pub source: Option<String>,
    pub vin: Option<String>,
    pub timestamp: Option<i64>,
}

// ── CommandError ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum CommandError {
    /// The payload is not valid JSON. Discard without publishing a response.
    InvalidJson(String),
    /// Valid JSON but a required field is missing or has an invalid value.
    InvalidCommand(String),
    /// The `doors` array contains a value other than "driver".
    UnsupportedDoor(String),
}

impl CommandError {
    /// The failure reason string published in command responses.
    pub fn reason(&self) -> &str {
        match self {
            CommandError::InvalidJson(_) => "invalid_json",
            CommandError::InvalidCommand(_) => "invalid_command",
            CommandError::UnsupportedDoor(_) => "unsupported_door",
        }
    }
}

// ── parse_command ─────────────────────────────────────────────────────────────

/// Deserialize a JSON command payload into a `LockCommand`.
///
/// - JSON syntax errors → `InvalidJson` (no response should be published)
/// - Missing/invalid required fields → `InvalidCommand`
pub fn parse_command(json: &str) -> Result<LockCommand, CommandError> {
    serde_json::from_str::<LockCommand>(json).map_err(|e| {
        if e.is_syntax() || e.is_eof() {
            CommandError::InvalidJson(e.to_string())
        } else {
            // Data errors: missing required fields, unknown enum variants, etc.
            CommandError::InvalidCommand(e.to_string())
        }
    })
}

// ── validate_command ──────────────────────────────────────────────────────────

/// Validate a parsed `LockCommand` for semantic correctness.
///
/// - `command_id` must be non-empty
/// - `doors` must contain only "driver" (and be non-empty)
pub fn validate_command(cmd: &LockCommand) -> Result<(), CommandError> {
    if cmd.command_id.is_empty() {
        return Err(CommandError::InvalidCommand(
            "command_id must not be empty".to_string(),
        ));
    }
    if cmd.doors.is_empty() {
        return Err(CommandError::UnsupportedDoor(
            "doors array must contain 'driver'".to_string(),
        ));
    }
    for door in &cmd.doors {
        if door != "driver" {
            return Err(CommandError::UnsupportedDoor(format!(
                "unsupported door: {door}"
            )));
        }
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-03-2: Deserialize a full lock command JSON with all fields.
    #[test]
    fn test_parse_valid_command() {
        let json = r#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}"#;
        let cmd = parse_command(json).expect("should parse valid lock command");
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, Action::Lock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, Some("companion_app".to_string()));
        assert_eq!(cmd.vin, Some("WDB123".to_string()));
        assert_eq!(cmd.timestamp, Some(1700000000));
    }

    /// TS-03-2: Deserialize an unlock command without optional fields.
    #[test]
    fn test_parse_valid_unlock_command() {
        let json = r#"{"command_id":"xyz-456","action":"unlock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse valid unlock command");
        assert_eq!(cmd.command_id, "xyz-456");
        assert_eq!(cmd.action, Action::Unlock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, None);
        assert_eq!(cmd.vin, None);
        assert_eq!(cmd.timestamp, None);
    }

    /// TS-03-4 / 03-REQ-2.3 / 03-REQ-2.E3: Empty command_id is rejected.
    #[test]
    fn test_validate_empty_command_id() {
        let json = r#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse (empty string is valid JSON)");
        let result = validate_command(&cmd);
        assert!(result.is_err(), "empty command_id must be rejected");
        assert_eq!(
            result.unwrap_err().reason(),
            "invalid_command",
            "reason must be 'invalid_command'"
        );
    }

    /// TS-03-4: Missing command_id field is rejected.
    #[test]
    fn test_parse_missing_command_id() {
        let json = r#"{"action":"lock","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "missing command_id must be rejected");
    }

    /// TS-03-5 / 03-REQ-2.1: Invalid action value is rejected by serde.
    #[test]
    fn test_validate_invalid_action() {
        let json = r#"{"command_id":"x","action":"toggle","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "invalid action 'toggle' must be rejected");
    }

    /// TS-03-6 / 03-REQ-2.2: Non-"driver" door is rejected with "unsupported_door".
    #[test]
    fn test_validate_unsupported_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["passenger"]}"#;
        let cmd = parse_command(json).expect("should parse (passenger is valid string)");
        let result = validate_command(&cmd);
        assert!(result.is_err(), "non-driver door must be rejected");
        assert_eq!(
            result.unwrap_err().reason(),
            "unsupported_door",
            "reason must be 'unsupported_door'"
        );
    }

    /// TS-03-E3 / 03-REQ-2.E1: Non-JSON payload returns InvalidJson error.
    #[test]
    fn test_parse_invalid_json() {
        let result = parse_command("not valid json {{{");
        assert!(result.is_err(), "invalid JSON must be rejected");
        assert!(
            matches!(result, Err(CommandError::InvalidJson(_))),
            "must be classified as InvalidJson"
        );
    }

    /// TS-03-E4 / 03-REQ-2.E2: Missing action field is rejected.
    #[test]
    fn test_parse_missing_field() {
        let json = r#"{"command_id":"x","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "missing action field must be rejected");
    }

    /// TS-03-E5 / 03-REQ-2.2: "rear_left" door is rejected with "unsupported_door".
    #[test]
    fn test_validate_non_driver_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["rear_left"]}"#;
        let cmd = parse_command(json).expect("should parse (rear_left is valid string)");
        let result = validate_command(&cmd);
        assert!(result.is_err(), "non-driver door must be rejected");
        assert_eq!(
            result.unwrap_err().reason(),
            "unsupported_door",
            "reason must be 'unsupported_door'"
        );
    }
}
