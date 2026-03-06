//! Command data types and validation logic.
//!
//! Handles parsing and validation of lock/unlock commands received via NATS.

use serde::{Deserialize, Serialize};

/// A lock/unlock command received from the CLOUD_GATEWAY via NATS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub command_id: String,
    pub action: String,
    pub doors: Vec<String>,
    pub source: String,
    pub vin: String,
    pub timestamp: i64,
}

/// Errors that can occur during command validation.
#[derive(Debug)]
pub enum CommandError {
    /// The JSON payload could not be parsed.
    ParseError(String),
    /// A required field is missing or invalid.
    ValidationError(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::ParseError(msg) => write!(f, "parse error: {msg}"),
            CommandError::ValidationError(msg) => write!(f, "validation error: {msg}"),
        }
    }
}

impl std::error::Error for CommandError {}

impl Command {
    /// Parse and validate a command from a JSON byte slice.
    ///
    /// Validates:
    /// - JSON is well-formed and contains all required fields
    /// - `action` is `"lock"` or `"unlock"`
    /// - `command_id` is a valid UUID string
    pub fn from_json(data: &[u8]) -> Result<Self, CommandError> {
        let cmd: Command = serde_json::from_slice(data)
            .map_err(|e| CommandError::ParseError(e.to_string()))?;

        // Validate action is "lock" or "unlock"
        if cmd.action != "lock" && cmd.action != "unlock" {
            return Err(CommandError::ValidationError(format!(
                "invalid action '{}': must be 'lock' or 'unlock'",
                cmd.action
            )));
        }

        // Validate command_id is a valid UUID
        uuid::Uuid::parse_str(&cmd.command_id).map_err(|e| {
            CommandError::ValidationError(format!(
                "invalid command_id '{}': not a valid UUID: {e}",
                cmd.command_id
            ))
        })?;

        Ok(cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_command_json() -> &'static str {
        r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#
    }

    /// TS-04-E1: Valid command JSON parses and validates successfully
    #[test]
    fn test_valid_command_parses() {
        let cmd = Command::from_json(valid_command_json().as_bytes())
            .expect("valid command should parse");
        assert_eq!(cmd.command_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(cmd.action, "lock");
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, "companion_app");
        assert_eq!(cmd.vin, "TEST_VIN_001");
        assert_eq!(cmd.timestamp, 1700000000);
    }

    /// TS-04-E1: Valid unlock command parses successfully
    #[test]
    fn test_valid_unlock_command_parses() {
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440001",
            "action": "unlock",
            "doors": ["driver", "passenger"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000001
        }"#;
        let cmd = Command::from_json(json.as_bytes()).expect("unlock command should parse");
        assert_eq!(cmd.action, "unlock");
    }

    /// TS-04-E1: Malformed JSON returns a parse error
    #[test]
    fn test_malformed_json_returns_parse_error() {
        let result = Command::from_json(b"not valid json {{{");
        assert!(result.is_err(), "malformed JSON should return an error");
        match result.unwrap_err() {
            CommandError::ParseError(_) => {} // expected
            other => panic!("expected ParseError, got: {other}"),
        }
    }

    /// TS-04-E2: JSON missing `action` field returns a validation error
    #[test]
    fn test_missing_action_field() {
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let result = Command::from_json(json.as_bytes());
        assert!(result.is_err(), "missing action should return an error");
    }

    /// TS-04-E2: JSON missing `command_id` field returns a validation error
    #[test]
    fn test_missing_command_id_field() {
        let json = r#"{
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let result = Command::from_json(json.as_bytes());
        assert!(result.is_err(), "missing command_id should return an error");
    }

    /// TS-04-E3: Invalid action value (not "lock" or "unlock") returns a validation error
    #[test]
    fn test_invalid_action_value() {
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "action": "reboot",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let result = Command::from_json(json.as_bytes());
        assert!(result.is_err(), "invalid action should return an error");
        match result.unwrap_err() {
            CommandError::ValidationError(msg) => {
                assert!(
                    msg.to_lowercase().contains("action"),
                    "error should mention action, got: {msg}"
                );
            }
            other => panic!("expected ValidationError, got: {other}"),
        }
    }

    /// TS-04-E3: Invalid command_id (not a UUID) returns a validation error
    #[test]
    fn test_invalid_command_id_not_uuid() {
        let json = r#"{
            "command_id": "not-a-uuid",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let result = Command::from_json(json.as_bytes());
        assert!(result.is_err(), "invalid UUID should return an error");
        match result.unwrap_err() {
            CommandError::ValidationError(msg) => {
                assert!(
                    msg.to_lowercase().contains("command_id")
                        || msg.to_lowercase().contains("uuid"),
                    "error should mention command_id or uuid, got: {msg}"
                );
            }
            other => panic!("expected ValidationError, got: {other}"),
        }
    }

    /// TS-04-E2: JSON missing `doors` field returns an error
    #[test]
    fn test_missing_doors_field() {
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "action": "lock",
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let result = Command::from_json(json.as_bytes());
        assert!(result.is_err(), "missing doors should return an error");
    }

    /// TS-04-E2: JSON missing `source` field returns an error
    #[test]
    fn test_missing_source_field() {
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "action": "lock",
            "doors": ["driver"],
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let result = Command::from_json(json.as_bytes());
        assert!(result.is_err(), "missing source should return an error");
    }

    /// TS-04-E2: JSON missing `vin` field returns an error
    #[test]
    fn test_missing_vin_field() {
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "timestamp": 1700000000
        }"#;
        let result = Command::from_json(json.as_bytes());
        assert!(result.is_err(), "missing vin should return an error");
    }

    /// TS-04-E2: JSON missing `timestamp` field returns an error
    #[test]
    fn test_missing_timestamp_field() {
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001"
        }"#;
        let result = Command::from_json(json.as_bytes());
        assert!(result.is_err(), "missing timestamp should return an error");
    }
}
