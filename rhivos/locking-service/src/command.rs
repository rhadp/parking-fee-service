use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Lock,
    Unlock,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LockCommand {
    pub command_id: String,
    pub action: Action,
    pub doors: Vec<String>,
    /// Optional metadata (03-REQ-2.4): deserialized but not used in processing.
    #[allow(dead_code)]
    pub source: Option<String>,
    /// Optional metadata (03-REQ-2.4): deserialized but not used in processing.
    #[allow(dead_code)]
    pub vin: Option<String>,
    /// Optional metadata (03-REQ-2.4): deserialized but not used in processing.
    #[allow(dead_code)]
    pub timestamp: Option<i64>,
}

#[derive(Debug)]
pub enum CommandError {
    InvalidJson(String),
    InvalidCommand(String),
    UnsupportedDoor(String),
}

impl CommandError {
    pub fn reason(&self) -> &str {
        match self {
            CommandError::InvalidJson(_) => "invalid_json",
            CommandError::InvalidCommand(_) => "invalid_command",
            CommandError::UnsupportedDoor(_) => "unsupported_door",
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::InvalidJson(msg) => write!(f, "invalid JSON: {msg}"),
            CommandError::InvalidCommand(msg) => write!(f, "invalid command: {msg}"),
            CommandError::UnsupportedDoor(msg) => write!(f, "unsupported door: {msg}"),
        }
    }
}

/// Deserialise a JSON string into a `LockCommand`.
///
/// Returns `InvalidJson` for syntactically invalid JSON, and `InvalidCommand`
/// for valid JSON that is missing required fields or has incorrect types.
pub fn parse_command(json: &str) -> Result<LockCommand, CommandError> {
    serde_json::from_str::<LockCommand>(json).map_err(|e| {
        // serde_json classifies syntax errors (unexpected token, EOF, etc.)
        // differently from data/type errors (missing field, wrong type).
        if e.classify() == serde_json::error::Category::Syntax {
            CommandError::InvalidJson(e.to_string())
        } else {
            CommandError::InvalidCommand(e.to_string())
        }
    })
}

/// Validate a parsed `LockCommand` for business rules:
/// - `command_id` must be non-empty
/// - `doors` must contain only `"driver"`
pub fn validate_command(cmd: &LockCommand) -> Result<(), CommandError> {
    if cmd.command_id.is_empty() {
        return Err(CommandError::InvalidCommand(
            "command_id must not be empty".to_string(),
        ));
    }
    if cmd.doors.iter().any(|d| d != "driver") {
        return Err(CommandError::UnsupportedDoor(format!(
            "unsupported door(s): {:?}",
            cmd.doors
                .iter()
                .filter(|d| d.as_str() != "driver")
                .collect::<Vec<_>>()
        )));
    }
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

    // TS-03-2: deserialise full lock command JSON
    #[test]
    fn test_parse_valid_command() {
        let json = r#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}"#;
        let cmd = parse_command(json).expect("should parse valid lock command");
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, Action::Lock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, Some("companion_app".to_string()));
        assert_eq!(cmd.vin, Some("WDB123".to_string()));
        assert_eq!(cmd.timestamp, Some(1_700_000_000));
    }

    // TS-03-2: deserialise unlock command
    #[test]
    fn test_parse_valid_unlock_command() {
        let json = r#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse valid unlock command");
        assert_eq!(cmd.command_id, "def-456");
        assert_eq!(cmd.action, Action::Unlock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, None);
        assert_eq!(cmd.vin, None);
        assert_eq!(cmd.timestamp, None);
    }

    // TS-03-4: empty command_id rejected
    #[test]
    fn test_validate_empty_command_id() {
        let json = r#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse JSON with empty command_id");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "invalid_command");
    }

    // TS-03-4: missing command_id field rejected
    #[test]
    fn test_parse_missing_command_id() {
        let json = r#"{"action":"lock","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "should reject JSON missing command_id");
    }

    // TS-03-5: invalid action rejected
    #[test]
    fn test_validate_invalid_action() {
        let json = r#"{"command_id":"x","action":"toggle","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "should reject invalid action 'toggle'");
    }

    // TS-03-6: non-"driver" door rejected
    #[test]
    fn test_validate_unsupported_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["passenger"]}"#;
        let cmd = parse_command(json).expect("should parse JSON with unsupported door");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }

    // TS-03-E3: invalid JSON returns InvalidJson error
    #[test]
    fn test_parse_invalid_json() {
        let result = parse_command("not valid json {{{");
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), CommandError::InvalidJson(_)),
            "should return InvalidJson variant for non-JSON input"
        );
    }

    // TS-03-E4: missing action field rejected
    #[test]
    fn test_parse_missing_field() {
        let json = r#"{"command_id":"x","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "should reject JSON missing action field");
    }

    // TS-03-E5: "rear_left" door rejected
    #[test]
    fn test_validate_non_driver_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["rear_left"]}"#;
        let cmd = parse_command(json).expect("should parse JSON with rear_left door");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }
}
