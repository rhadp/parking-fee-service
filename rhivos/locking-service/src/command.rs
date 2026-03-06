use serde::{Deserialize, Serialize};

/// Represents a lock/unlock command received from DATA_BROKER.
#[derive(Debug, Deserialize)]
pub struct Command {
    pub command_id: String,
    pub action: String,
    pub doors: Vec<String>,
    pub source: String,
    pub vin: String,
    pub timestamp: u64,
}

/// Validation error types for command parsing.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    /// JSON could not be parsed.
    MalformedJson(String),
    /// A required field is missing.
    MissingField(String),
    /// The action value is not "lock" or "unlock".
    InvalidAction(String),
}

/// Response to a lock/unlock command.
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: u64,
}

impl Command {
    /// Parse and validate a command from a JSON string.
    ///
    /// Returns `Ok(Command)` if the JSON is well-formed and all required fields
    /// are present and valid. Returns `Err(ValidationError)` otherwise.
    ///
    /// Validation order:
    /// 1. Parse JSON structure
    /// 2. Check all required fields are present and non-empty
    /// 3. Validate `action` is `"lock"` or `"unlock"`
    pub fn from_json(json_str: &str) -> Result<Self, ValidationError> {
        // Step 1: Parse as generic JSON value first to give better field-level errors
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| ValidationError::MalformedJson(e.to_string()))?;

        let obj = value
            .as_object()
            .ok_or_else(|| ValidationError::MalformedJson("expected JSON object".to_string()))?;

        // Step 2: Check required fields are present and non-empty
        Self::check_required_string(obj, "command_id")?;
        Self::check_required_string(obj, "action")?;
        Self::check_required_string(obj, "source")?;
        Self::check_required_string(obj, "vin")?;

        if !obj.contains_key("timestamp") {
            return Err(ValidationError::MissingField("timestamp".to_string()));
        }

        if !obj.contains_key("doors") {
            return Err(ValidationError::MissingField("doors".to_string()));
        }

        // Step 3: Validate action value
        let action = obj["action"].as_str().unwrap();
        if action != "lock" && action != "unlock" {
            return Err(ValidationError::InvalidAction(action.to_string()));
        }

        // Now deserialize into the typed struct (all fields validated)
        let cmd: Command = serde_json::from_value(value)
            .map_err(|e| ValidationError::MalformedJson(e.to_string()))?;

        Ok(cmd)
    }

    /// Check that a required string field is present and non-empty.
    fn check_required_string(
        obj: &serde_json::Map<String, serde_json::Value>,
        field: &str,
    ) -> Result<(), ValidationError> {
        match obj.get(field) {
            None => Err(ValidationError::MissingField(field.to_string())),
            Some(v) => match v.as_str() {
                Some(s) if !s.is_empty() => Ok(()),
                Some(_) => Err(ValidationError::MissingField(field.to_string())),
                None => Err(ValidationError::MissingField(field.to_string())),
            },
        }
    }
}

impl CommandResponse {
    /// Create a success response.
    pub fn success(command_id: String, timestamp: u64) -> Self {
        CommandResponse {
            command_id,
            status: "success".to_string(),
            reason: None,
            timestamp,
        }
    }

    /// Create a failure response.
    pub fn failure(command_id: String, reason: String, timestamp: u64) -> Self {
        CommandResponse {
            command_id,
            status: "failed".to_string(),
            reason: Some(reason),
            timestamp,
        }
    }

    /// Serialize the response to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("CommandResponse serialization should not fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- TS-03-E4: Command parsing tests ---

    #[test]
    fn test_valid_lock_command_parses() {
        let json = r#"{
            "command_id": "550e8400-e29b-41d4-a716-446655440000",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000000
        }"#;
        let cmd = Command::from_json(json);
        assert!(cmd.is_ok(), "Valid lock command should parse successfully");
        let cmd = cmd.unwrap();
        assert_eq!(cmd.action, "lock");
        assert_eq!(cmd.command_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, "companion_app");
        assert_eq!(cmd.vin, "TEST_VIN_001");
        assert_eq!(cmd.timestamp, 1700000000);
    }

    #[test]
    fn test_valid_unlock_command_parses() {
        let json = r#"{
            "command_id": "660e8400-e29b-41d4-a716-446655440001",
            "action": "unlock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000010
        }"#;
        let cmd = Command::from_json(json);
        assert!(cmd.is_ok(), "Valid unlock command should parse successfully");
        let cmd = cmd.unwrap();
        assert_eq!(cmd.action, "unlock");
    }

    /// TS-03-E1: Malformed JSON returns a parse error.
    #[test]
    fn test_malformed_json_returns_error() {
        let json = "not valid json {{{";
        let result = Command::from_json(json);
        assert!(result.is_err(), "Malformed JSON should return an error");
        match result.unwrap_err() {
            ValidationError::MalformedJson(_) => {} // expected
            other => panic!("Expected MalformedJson, got {:?}", other),
        }
    }

    /// TS-03-E2: Missing 'action' field returns a validation error.
    #[test]
    fn test_missing_action_field() {
        let json = r#"{
            "command_id": "990e8400-e29b-41d4-a716-446655440004",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000040
        }"#;
        let result = Command::from_json(json);
        assert!(result.is_err(), "Missing action field should return an error");
        match result.unwrap_err() {
            ValidationError::MissingField(field) => {
                assert_eq!(field, "action", "Should identify 'action' as missing field");
            }
            other => panic!("Expected MissingField, got {:?}", other),
        }
    }

    /// TS-03-E2: Missing 'command_id' field returns a validation error.
    #[test]
    fn test_missing_command_id_field() {
        let json = r#"{
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000040
        }"#;
        let result = Command::from_json(json);
        assert!(result.is_err(), "Missing command_id field should return an error");
        match result.unwrap_err() {
            ValidationError::MissingField(field) => {
                assert_eq!(field, "command_id", "Should identify 'command_id' as missing field");
            }
            other => panic!("Expected MissingField, got {:?}", other),
        }
    }

    /// TS-03-E3: Invalid action value returns a validation error.
    #[test]
    fn test_invalid_action_value() {
        let json = r#"{
            "command_id": "aa0e8400-e29b-41d4-a716-446655440005",
            "action": "reboot",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000050
        }"#;
        let result = Command::from_json(json);
        assert!(result.is_err(), "Invalid action should return an error");
        match result.unwrap_err() {
            ValidationError::InvalidAction(action) => {
                assert_eq!(action, "reboot", "Should identify 'reboot' as invalid action");
            }
            other => panic!("Expected InvalidAction, got {:?}", other),
        }
    }

    // --- TS-03-E4: Response serialization tests ---

    #[test]
    fn test_success_response_serialization() {
        let response = CommandResponse::success(
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
            1700000000,
        );
        let json_str = response.to_json();
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .expect("Success response should be valid JSON");

        assert_eq!(value["command_id"], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(value["status"], "success");
        assert!(value.get("reason").is_none(), "Success response should not have a reason field");
        assert_eq!(value["timestamp"], 1700000000);
    }

    #[test]
    fn test_failure_response_serialization() {
        let response = CommandResponse::failure(
            "770e8400-e29b-41d4-a716-446655440002".to_string(),
            "vehicle_moving".to_string(),
            1700000020,
        );
        let json_str = response.to_json();
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .expect("Failure response should be valid JSON");

        assert_eq!(value["command_id"], "770e8400-e29b-41d4-a716-446655440002");
        assert_eq!(value["status"], "failed");
        assert_eq!(value["reason"], "vehicle_moving");
        assert_eq!(value["timestamp"], 1700000020);
    }
}
