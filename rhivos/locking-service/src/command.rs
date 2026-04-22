use serde::Deserialize;

/// Action to perform on the door lock.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Lock the door.
    Lock,
    /// Unlock the door.
    Unlock,
}

/// A lock/unlock command received from DATA_BROKER.
#[derive(Debug, Clone, Deserialize)]
pub struct LockCommand {
    /// Unique command identifier (required, non-empty).
    pub command_id: String,
    /// The action to perform (required).
    pub action: Action,
    /// Target doors (required, must contain "driver").
    pub doors: Vec<String>,
    /// Optional source metadata.
    pub source: Option<String>,
    /// Optional VIN metadata.
    pub vin: Option<String>,
    /// Optional timestamp metadata.
    pub timestamp: Option<i64>,
}

/// Error type for command parsing and validation.
#[derive(Debug)]
pub enum CommandError {
    /// The payload is not valid JSON.
    InvalidJson(String),
    /// A required field is missing or invalid.
    InvalidCommand(String),
    /// An unsupported door value was specified.
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

/// Parse a JSON string into a LockCommand.
pub fn parse_command(_json: &str) -> Result<LockCommand, CommandError> {
    todo!()
}

/// Validate a parsed LockCommand for business rules.
pub fn validate_command(_cmd: &LockCommand) -> Result<(), CommandError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-2: Verify a valid lock command JSON is deserialized into a LockCommand
    // with all fields correctly populated.
    #[test]
    fn test_parse_valid_command() {
        let json = r#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}"#;
        let cmd = parse_command(json).expect("should parse valid command");
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, Action::Lock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source.as_deref(), Some("companion_app"));
    }

    // TS-03-2: Verify a valid unlock command JSON is deserialized correctly.
    #[test]
    fn test_parse_valid_unlock_command() {
        let json = r#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;
        let cmd = parse_command(json).expect("should parse valid unlock command");
        assert_eq!(cmd.command_id, "def-456");
        assert_eq!(cmd.action, Action::Unlock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert!(cmd.source.is_none());
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

    // TS-03-5: Verify that an invalid action value (e.g. "toggle") is rejected.
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
        let cmd = parse_command(json).expect("should parse JSON");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }

    // TS-03-E3: Verify that non-JSON payloads are discarded (InvalidJson error).
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

    // TS-03-E5: Verify that "rear_left" door is rejected with "unsupported_door".
    #[test]
    fn test_validate_non_driver_door() {
        let json = r#"{"command_id":"x","action":"lock","doors":["rear_left"]}"#;
        let cmd = parse_command(json).expect("should parse JSON");
        let result = validate_command(&cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().reason(), "unsupported_door");
    }
}
