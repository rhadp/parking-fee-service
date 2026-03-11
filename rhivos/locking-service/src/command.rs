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

/// Error types for command validation.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    /// JSON parsing failed.
    MalformedJson(String),
    /// A required field is missing or empty.
    MissingField(String),
    /// The action value is not "lock" or "unlock".
    InvalidAction(String),
}

/// Represents the response written to DATA_BROKER after command processing.
#[derive(Debug, Serialize, Deserialize)]
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
    /// Returns the parsed command or a validation error.
    pub fn from_json(json_str: &str) -> Result<Self, ValidationError> {
        let cmd: Command = serde_json::from_str(json_str)
            .map_err(|e| ValidationError::MalformedJson(e.to_string()))?;

        // Validate required fields are non-empty.
        if cmd.command_id.is_empty() {
            return Err(ValidationError::MissingField("command_id".to_string()));
        }
        if cmd.action.is_empty() {
            return Err(ValidationError::MissingField("action".to_string()));
        }
        if cmd.doors.is_empty() {
            return Err(ValidationError::MissingField("doors".to_string()));
        }
        if cmd.source.is_empty() {
            return Err(ValidationError::MissingField("source".to_string()));
        }
        if cmd.vin.is_empty() {
            return Err(ValidationError::MissingField("vin".to_string()));
        }

        Ok(cmd)
    }

    /// Validate the action field is "lock" or "unlock".
    pub fn validate_action(&self) -> Result<(), ValidationError> {
        match self.action.as_str() {
            "lock" | "unlock" => Ok(()),
            other => Err(ValidationError::InvalidAction(other.to_string())),
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
        // TS-03-1: Valid lock command JSON parses successfully
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
    }

    #[test]
    fn test_valid_unlock_command_parses() {
        // TS-03-2: Valid unlock command JSON parses successfully
        let json = r#"{
            "command_id": "660e8400-e29b-41d4-a716-446655440001",
            "action": "unlock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000010
        }"#;
        let cmd = Command::from_json(json).expect("Valid unlock command should parse");
        assert_eq!(cmd.action, "unlock");
    }

    #[test]
    fn test_malformed_json_returns_error() {
        // TS-03-E1: Malformed JSON returns a parse error
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
        // TS-03-E2: JSON missing `action` field returns a validation error
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
            ValidationError::MalformedJson(_) | ValidationError::MissingField(_) => {}
            other => panic!("Expected MalformedJson or MissingField error, got {:?}", other),
        }
    }

    #[test]
    fn test_missing_command_id_field_returns_error() {
        // TS-03-E2: JSON missing `command_id` field returns a validation error
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
            ValidationError::MalformedJson(_) | ValidationError::MissingField(_) => {}
            other => panic!("Expected MalformedJson or MissingField error, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_action_returns_error() {
        // TS-03-E3: JSON with invalid `action` value returns a validation error
        let json = r#"{
            "command_id": "aa0e8400-e29b-41d4-a716-446655440005",
            "action": "reboot",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "TEST_VIN_001",
            "timestamp": 1700000050
        }"#;
        // from_json should parse successfully (fields are present), but validate_action should fail
        let cmd = Command::from_json(json).expect("JSON with all fields should parse");
        let result = cmd.validate_action();
        assert!(result.is_err(), "Invalid action 'reboot' should be rejected");
        match result.unwrap_err() {
            ValidationError::InvalidAction(action) => {
                assert_eq!(action, "reboot");
            }
            other => panic!("Expected InvalidAction error, got {:?}", other),
        }
    }

    // --- TS-03-E4: Response format tests ---

    #[test]
    fn test_success_response_serialization() {
        // TS-03-E4: Success response serializes to the expected JSON format
        let response = CommandResponse::success(
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
            1700000000,
        );
        assert_eq!(response.command_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(response.status, "success");
        assert!(response.reason.is_none(), "Success response should not have a reason field");
        assert_eq!(response.timestamp, 1700000000);

        let json_str = response.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .expect("Response JSON should be valid");
        assert_eq!(parsed["status"], "success");
        assert_eq!(parsed["command_id"], "550e8400-e29b-41d4-a716-446655440000");
        assert!(parsed.get("reason").is_none(), "Success JSON should not contain reason field");
        assert!(parsed["timestamp"].is_u64(), "Timestamp should be a non-negative integer");
    }

    #[test]
    fn test_failure_response_serialization() {
        // TS-03-E4: Failure response serializes to the expected JSON format
        let response = CommandResponse::failure(
            "770e8400-e29b-41d4-a716-446655440002".to_string(),
            "vehicle_moving".to_string(),
            1700000020,
        );
        assert_eq!(response.command_id, "770e8400-e29b-41d4-a716-446655440002");
        assert_eq!(response.status, "failed");
        assert_eq!(response.reason, Some("vehicle_moving".to_string()));
        assert_eq!(response.timestamp, 1700000020);

        let json_str = response.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .expect("Response JSON should be valid");
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["command_id"], "770e8400-e29b-41d4-a716-446655440002");
        assert_eq!(parsed["reason"], "vehicle_moving");
        assert!(parsed["timestamp"].is_u64(), "Timestamp should be a non-negative integer");
    }
}
