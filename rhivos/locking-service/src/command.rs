use serde::Deserialize;
use std::fmt;

/// Lock/unlock action.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Lock,
    Unlock,
}

/// A lock/unlock command received from CLOUD_GATEWAY_CLIENT.
#[derive(Debug, Clone, Deserialize)]
pub struct LockCommand {
    pub command_id: String,
    pub action: Action,
    pub doors: Vec<String>,
    pub source: Option<String>,
    pub vin: Option<String>,
    pub timestamp: Option<i64>,
}

/// Errors that can occur during command parsing or validation.
#[derive(Debug)]
pub enum CommandError {
    /// The payload is not valid JSON.
    InvalidJson(String),
    /// A required field is missing or invalid.
    InvalidCommand(String),
    /// The doors array contains an unsupported door value.
    UnsupportedDoor(String),
}

impl CommandError {
    /// Returns the reason string for response publishing.
    pub fn reason(&self) -> &str {
        match self {
            CommandError::InvalidJson(_) => "invalid_json",
            CommandError::InvalidCommand(_) => "invalid_command",
            CommandError::UnsupportedDoor(_) => "unsupported_door",
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::InvalidJson(msg) => write!(f, "invalid JSON: {msg}"),
            CommandError::InvalidCommand(msg) => write!(f, "invalid command: {msg}"),
            CommandError::UnsupportedDoor(msg) => write!(f, "unsupported door: {msg}"),
        }
    }
}

impl std::error::Error for CommandError {}

/// Parse a JSON string into a `LockCommand`.
///
/// Returns `InvalidJson` if the string is not valid JSON.
/// Returns `InvalidCommand` if required fields are missing or have wrong types.
pub fn parse_command(json: &str) -> Result<LockCommand, CommandError> {
    serde_json::from_str(json).map_err(|e| {
        if e.is_syntax() || e.is_eof() {
            CommandError::InvalidJson(e.to_string())
        } else {
            CommandError::InvalidCommand(e.to_string())
        }
    })
}

/// Validate a parsed `LockCommand`.
///
/// Returns `InvalidCommand` if `command_id` is empty or `doors` does not contain "driver".
/// Returns `UnsupportedDoor` if `doors` contains any value other than "driver".
pub fn validate_command(cmd: &LockCommand) -> Result<(), CommandError> {
    if cmd.command_id.is_empty() {
        return Err(CommandError::InvalidCommand(
            "command_id is empty".to_string(),
        ));
    }
    // Check for unsupported door values first (03-REQ-2.2).
    for door in &cmd.doors {
        if door != "driver" {
            return Err(CommandError::UnsupportedDoor(door.clone()));
        }
    }
    // Verify "driver" is present in the array (03-REQ-2.1).
    if !cmd.doors.contains(&"driver".to_string()) {
        return Err(CommandError::InvalidCommand(
            "doors must contain \"driver\"".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-2: Verify a valid lock command JSON is deserialized correctly.
    #[test]
    fn test_parse_valid_command() {
        let json = r#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}"#;
        let cmd = parse_command(json).expect("should parse valid command");
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, Action::Lock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, Some("companion_app".to_string()));
    }

    // TS-03-2: Verify a valid unlock command JSON is deserialized correctly.
    #[test]
    fn test_parse_valid_unlock_command() {
        let json = r#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse valid unlock command");
        assert_eq!(cmd.command_id, "def-456");
        assert_eq!(cmd.action, Action::Unlock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, None);
    }

    // TS-03-4: Verify that an empty command_id is rejected with reason "invalid_command".
    #[test]
    fn test_validate_empty_command_id() {
        let json = r#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse JSON with empty command_id");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "invalid_command");
    }

    // TS-03-4: Verify that a missing command_id field is rejected.
    #[test]
    fn test_parse_missing_command_id() {
        let json = r#"{"action":"lock","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err());
    }

    // TS-03-5: Verify that an invalid action value is rejected.
    #[test]
    fn test_validate_invalid_action() {
        let json = r#"{"command_id":"x","action":"toggle","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err());
    }

    // TS-03-6: Verify that a non-"driver" door value is rejected with "unsupported_door".
    #[test]
    fn test_validate_unsupported_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["passenger"]}"#;
        let cmd = parse_command(json).expect("should parse JSON with unsupported door");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }

    // TS-03-E3: Verify that non-JSON payloads are discarded.
    #[test]
    fn test_parse_invalid_json() {
        let result = parse_command("not valid json {{{");
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::InvalidJson(_))));
    }

    // TS-03-E4: Verify that a payload missing the action field is rejected.
    #[test]
    fn test_parse_missing_field() {
        let json = r#"{"command_id":"x","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err());
    }

    // TS-03-E5: Verify that a non-"driver" door value results in "unsupported_door".
    #[test]
    fn test_validate_non_driver_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["rear_left"]}"#;
        let cmd = parse_command(json).expect("should parse JSON with non-driver door");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }

    // 03-REQ-2.1: Verify that an empty doors array is rejected.
    // The doors field must contain "driver"; an empty array does not satisfy this.
    #[test]
    fn test_validate_empty_doors_array() {
        let json = r#"{"command_id":"x","action":"lock","doors":[]}"#;
        let cmd = parse_command(json).expect("should parse JSON with empty doors");
        let result = validate_command(&cmd);
        assert!(result.is_err(), "empty doors array should be rejected");
        assert_eq!(result.unwrap_err().reason(), "invalid_command");
    }
}
