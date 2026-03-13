use serde::Deserialize;

/// Deserialized lock/unlock command from Vehicle.Command.Door.Lock
#[derive(Debug, Clone, Deserialize)]
pub struct LockCommand {
    pub command_id: String,
    pub action: Action,
    pub doors: Vec<String>,
    pub source: Option<String>,
    pub vin: Option<String>,
    pub timestamp: Option<i64>,
}

/// Lock or unlock action
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Lock,
    Unlock,
}

/// Error type for command parsing and validation
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    /// JSON parsing failed
    InvalidJson(String),
    /// Required field missing or invalid
    InvalidCommand(String),
    /// Door value not supported
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

/// Parse a JSON string into a LockCommand.
pub fn parse_command(json: &str) -> Result<LockCommand, CommandError> {
    serde_json::from_str::<LockCommand>(json)
        .map_err(|e| CommandError::InvalidJson(e.to_string()))
}

/// Validate a parsed LockCommand.
pub fn validate_command(cmd: &LockCommand) -> Result<(), CommandError> {
    if cmd.command_id.is_empty() {
        return Err(CommandError::InvalidCommand(
            "command_id must be non-empty".to_string(),
        ));
    }
    if !cmd.doors.contains(&"driver".to_string()) {
        return Err(CommandError::UnsupportedDoor(
            "doors must contain 'driver'".to_string(),
        ));
    }
    for door in &cmd.doors {
        if door != "driver" {
            return Err(CommandError::UnsupportedDoor(format!(
                "unsupported door: {}",
                door
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-2: Command Deserialization
    #[test]
    fn test_parse_valid_command() {
        let json = r#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}"#;
        let cmd = parse_command(json).expect("should parse valid command");
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, Action::Lock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source.as_deref(), Some("companion_app"));
        assert_eq!(cmd.vin.as_deref(), Some("WDB123"));
        assert_eq!(cmd.timestamp, Some(1700000000));
    }

    // TS-03-4: Validate command_id Required
    #[test]
    fn test_validate_empty_command_id() {
        let json = r#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse JSON");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "invalid_command");
    }

    // TS-03-5: Validate Action Field
    #[test]
    fn test_validate_invalid_action() {
        let json = r#"{"command_id":"x","action":"toggle","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "toggle is not a valid action");
    }

    // TS-03-6: Validate Doors Field
    #[test]
    fn test_validate_unsupported_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["passenger"]}"#;
        let cmd = parse_command(json).expect("should parse JSON");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }

    // TS-03-E3: Invalid JSON Payload
    #[test]
    fn test_parse_invalid_json() {
        let result = parse_command("not valid json {{{");
        assert!(result.is_err());
    }

    // TS-03-E4: Missing Required Field
    #[test]
    fn test_parse_missing_field() {
        let json = r#"{"command_id":"x","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "missing action field should fail");
    }

    // TS-03-E5: Unsupported Door Value
    #[test]
    fn test_validate_non_driver_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["rear_left"]}"#;
        let cmd = parse_command(json).expect("should parse JSON");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }

    // TS-03-2 additional: parse valid unlock command
    #[test]
    fn test_parse_valid_unlock_command() {
        let json = r#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse valid unlock command");
        assert_eq!(cmd.command_id, "def-456");
        assert_eq!(cmd.action, Action::Unlock);
        assert_eq!(cmd.doors, vec!["driver"]);
    }

    // TS-03-4 additional: missing command_id field entirely
    #[test]
    fn test_parse_missing_command_id() {
        let json = r#"{"action":"lock","doors":["driver"]}"#;
        let result = parse_command(json);
        assert!(result.is_err(), "missing command_id field should fail");
    }
}
