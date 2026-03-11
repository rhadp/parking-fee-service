use serde::{Deserialize, Serialize};

/// Represents a lock/unlock command received from NATS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub command_id: String,
    pub action: String,
    pub doors: Vec<String>,
    pub source: String,
    pub vin: String,
    pub timestamp: u64,
}

/// Error types for command validation.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    /// JSON parsing failed.
    MalformedJson(String),
    /// A required field is missing or empty.
    MissingField(String),
    /// The action value is not "lock" or "unlock".
    InvalidAction(String),
    /// The command_id is not a valid UUID.
    InvalidCommandId(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MalformedJson(msg) => write!(f, "Malformed JSON: {}", msg),
            ValidationError::MissingField(field) => write!(f, "Missing required field: {}", field),
            ValidationError::InvalidAction(action) => write!(f, "Invalid action: {}", action),
            ValidationError::InvalidCommandId(id) => write!(f, "Invalid command_id: {}", id),
        }
    }
}

impl Command {
    /// Parse and validate a command from a JSON string.
    ///
    /// Returns the parsed command or a validation error.
    pub fn from_json(json_str: &str) -> Result<Self, ValidationError> {
        let cmd: Command =
            serde_json::from_str(json_str).map_err(|e| ValidationError::MalformedJson(e.to_string()))?;
        Ok(cmd)
    }

    /// Validate the action field is "lock" or "unlock".
    pub fn validate_action(&self) -> Result<(), ValidationError> {
        if self.action != "lock" && self.action != "unlock" {
            return Err(ValidationError::InvalidAction(self.action.clone()));
        }
        Ok(())
    }

    /// Validate the command_id field is a valid UUID.
    pub fn validate_command_id(&self) -> Result<(), ValidationError> {
        uuid::Uuid::parse_str(&self.command_id)
            .map_err(|_| ValidationError::InvalidCommandId(self.command_id.clone()))?;
        Ok(())
    }

    /// Run all validations on the command.
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.validate_command_id()?;
        self.validate_action()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- TS-04-E1, TS-04-E2, TS-04-E3: Command validation tests ---

    #[test]
    fn test_valid_lock_command_parses() {
        // TS-04-E1: Valid lock command JSON parses and validates successfully
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let cmd = Command::from_json(json).expect("Valid lock command should parse");
        assert_eq!(cmd.command_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(cmd.action, "lock");
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, "companion_app");
        assert_eq!(cmd.vin, "TEST_VIN_001");
        assert_eq!(cmd.timestamp, 1700000000);
        cmd.validate().expect("Valid lock command should pass validation");
    }

    #[test]
    fn test_valid_unlock_command_parses() {
        // Valid unlock command JSON parses and validates successfully
        let json = r#"{
            "command_id": "660e8400-e29b-41d4-a716-446655440001",
            "action": "unlock",
            "doors": ["driver", "passenger"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000010
        }"#;
        let cmd = Command::from_json(json).expect("Valid unlock command should parse");
        assert_eq!(cmd.action, "unlock");
        cmd.validate().expect("Valid unlock command should pass validation");
    }

    #[test]
    fn test_malformed_json_returns_error() {
        // TS-04-E1: Malformed JSON returns a parse error
        let json = "not valid json {{{";
        let result = Command::from_json(json);
        assert!(result.is_err(), "Malformed JSON should return an error");
        match result.unwrap_err() {
            ValidationError::MalformedJson(_) => {}
            other => panic!("Expected MalformedJson error, got {:?}", other),
        }
    }

    #[test]
    fn test_missing_action_field_returns_error() {
        // TS-04-E2: JSON missing `action` field returns a validation error
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let result = Command::from_json(json);
        assert!(
            result.is_err(),
            "Missing action field should return an error"
        );
        match result.unwrap_err() {
            ValidationError::MalformedJson(_) | ValidationError::MissingField(_) => {}
            other => panic!(
                "Expected MalformedJson or MissingField error, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_missing_command_id_field_returns_error() {
        // TS-04-E2: JSON missing `command_id` field returns a validation error
        let json = r#"{
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let result = Command::from_json(json);
        assert!(
            result.is_err(),
            "Missing command_id field should return an error"
        );
        match result.unwrap_err() {
            ValidationError::MalformedJson(_) | ValidationError::MissingField(_) => {}
            other => panic!(
                "Expected MalformedJson or MissingField error, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_invalid_action_returns_error() {
        // TS-04-E3: JSON with invalid `action` value returns a validation error
        let json = r#"{
            "command_id": "aa0e8400-e29b-41d4-a716-446655440005",
            "action": "reboot",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000050
        }"#;
        // from_json should parse successfully (all fields present), but validate should fail
        let cmd = Command::from_json(json).expect("JSON with all fields should parse");
        let result = cmd.validate_action();
        assert!(
            result.is_err(),
            "Invalid action 'reboot' should be rejected"
        );
        match result.unwrap_err() {
            ValidationError::InvalidAction(action) => {
                assert_eq!(action, "reboot");
            }
            other => panic!("Expected InvalidAction error, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_command_id_not_uuid() {
        // TS-04-E2: command_id that is not a valid UUID returns a validation error
        let json = r#"{
            "command_id": "not-a-uuid",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let cmd = Command::from_json(json).expect("JSON with all fields should parse");
        let result = cmd.validate_command_id();
        assert!(
            result.is_err(),
            "Invalid command_id (not a UUID) should be rejected"
        );
        match result.unwrap_err() {
            ValidationError::InvalidCommandId(_) => {}
            other => panic!("Expected InvalidCommandId error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_catches_invalid_action() {
        // Full validate() should catch invalid action
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "action": "destroy",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let cmd = Command::from_json(json).expect("JSON with all fields should parse");
        let result = cmd.validate();
        assert!(result.is_err(), "validate() should reject invalid action");
    }
}
