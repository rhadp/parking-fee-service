//! Command parsing and validation for lock/unlock commands.
use serde::Deserialize;

/// A parsed lock/unlock command.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct LockCommand {
    pub command_id: String,
    pub action: Action,
    pub doors: Vec<String>,
    pub source: Option<String>,
    pub vin: Option<String>,
    pub timestamp: Option<i64>,
}

/// The requested locking action.
#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Lock,
    Unlock,
}

/// Errors from command parsing or validation.
#[derive(Debug, PartialEq)]
pub enum CommandError {
    /// The payload is not valid JSON.
    InvalidJson(String),
    /// Required field is missing or has an invalid value.
    InvalidCommand(String),
    /// A door value other than "driver" was specified.
    UnsupportedDoor(String),
}

impl CommandError {
    /// Returns the failure reason string for response publishing.
    pub fn reason(&self) -> &str {
        match self {
            CommandError::InvalidJson(_) => "invalid_json",
            CommandError::InvalidCommand(_) => "invalid_command",
            CommandError::UnsupportedDoor(_) => "unsupported_door",
        }
    }
}

/// Parse a JSON string into a LockCommand.
///
/// Uses a two-phase approach:
/// 1. Parse as `serde_json::Value` — syntax errors → `InvalidJson`
/// 2. Extract required fields — missing/bad fields → `InvalidCommand`
///
/// This distinguishes malformed JSON from structurally invalid payloads.
pub fn parse_command(json: &str) -> Result<LockCommand, CommandError> {
    // Phase 1: parse as JSON value — catches syntax errors
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| CommandError::InvalidJson(e.to_string()))?;

    let obj = value
        .as_object()
        .ok_or_else(|| CommandError::InvalidCommand("payload must be a JSON object".to_string()))?;

    // command_id: required string
    let command_id = obj
        .get("command_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CommandError::InvalidCommand("missing or invalid command_id".to_string()))?
        .to_string();

    // action: required, must be "lock" or "unlock"
    let action_str = obj
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CommandError::InvalidCommand("missing or invalid action".to_string()))?;
    let action = match action_str {
        "lock" => Action::Lock,
        "unlock" => Action::Unlock,
        other => {
            return Err(CommandError::InvalidCommand(format!(
                "unknown action: {other}"
            )))
        }
    };

    // doors: required array of strings
    let doors = obj
        .get("doors")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CommandError::InvalidCommand("missing or invalid doors".to_string()))?
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();

    // Optional metadata fields
    let source = obj
        .get("source")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let vin = obj
        .get("vin")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let timestamp = obj.get("timestamp").and_then(|v| v.as_i64());

    Ok(LockCommand {
        command_id,
        action,
        doors,
        source,
        vin,
        timestamp,
    })
}

/// Validate a parsed LockCommand.
///
/// Checks:
/// - `command_id` is non-empty → `InvalidCommand`
/// - `doors` contains "driver" → `InvalidCommand` (03-REQ-2.1)
/// - all `doors` values are "driver" → `UnsupportedDoor` (03-REQ-2.2)
pub fn validate_command(cmd: &LockCommand) -> Result<(), CommandError> {
    if cmd.command_id.is_empty() {
        return Err(CommandError::InvalidCommand(
            "command_id must not be empty".to_string(),
        ));
    }

    // Check for unsupported doors first (03-REQ-2.2 takes priority over 2.1
    // for arrays containing non-"driver" values).
    for door in &cmd.doors {
        if door != "driver" {
            return Err(CommandError::UnsupportedDoor(format!(
                "unsupported door: {door}"
            )));
        }
    }

    // Then ensure "driver" is actually present (03-REQ-2.1).
    // This catches the empty doors array case.
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
        let cmd = parse_command(json).expect("should parse valid command");
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, Action::Lock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, Some("companion_app".to_string()));
        assert_eq!(cmd.vin, Some("WDB123".to_string()));
        assert_eq!(cmd.timestamp, Some(1700000000));
    }

    // TS-03-2: Deserialise unlock command
    #[test]
    fn test_parse_valid_unlock_command() {
        let json = r#"{"command_id":"u-1","action":"unlock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse unlock command");
        assert_eq!(cmd.action, Action::Unlock);
        assert_eq!(cmd.command_id, "u-1");
        assert!(cmd.source.is_none());
    }

    // TS-03-4: Empty command_id rejected with "invalid_command"
    #[test]
    fn test_validate_empty_command_id() {
        let json = r#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "invalid_command");
    }

    // TS-03-4: Missing command_id field rejected
    #[test]
    fn test_parse_missing_command_id() {
        let json = r#"{"action":"lock","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "missing command_id should fail");
    }

    // TS-03-5: Invalid action value rejected
    #[test]
    fn test_validate_invalid_action() {
        let json = r#"{"command_id":"x","action":"toggle","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "unknown action should fail");
    }

    // TS-03-6: Non-"driver" door rejected with "unsupported_door"
    #[test]
    fn test_validate_unsupported_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["passenger"]}"#;
        let cmd = parse_command(json).expect("should parse");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }

    // 03-REQ-2.1: Empty doors array rejected with "invalid_command"
    // Addresses minor review finding: empty doors passes validation but violates REQ-2.1
    #[test]
    fn test_validate_empty_doors() {
        let json = r#"{"command_id":"x","action":"lock","doors":[]}"#;
        let cmd = parse_command(json).expect("should parse");
        let result = validate_command(&cmd);
        assert!(result.is_err(), "empty doors must be rejected");
        assert_eq!(result.unwrap_err().reason(), "invalid_command");
    }

    // TS-03-E3: Invalid JSON returns InvalidJson error
    #[test]
    fn test_parse_invalid_json() {
        let result = parse_command("not valid json {{{");
        assert!(result.is_err());
        assert!(
            matches!(result, Err(CommandError::InvalidJson(_))),
            "expected InvalidJson, got: {result:?}"
        );
    }

    // TS-03-E4: Missing action field returns error
    #[test]
    fn test_parse_missing_field() {
        let json = r#"{"command_id":"x","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "missing action field should fail");
    }

    // TS-03-E5: "rear_left" door rejected with "unsupported_door"
    #[test]
    fn test_validate_non_driver_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["rear_left"]}"#;
        let cmd = parse_command(json).expect("should parse");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }
}
