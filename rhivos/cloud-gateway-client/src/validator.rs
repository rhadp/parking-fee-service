//! Command validation for cloud-gateway-client.
//!
//! This module validates incoming JSON commands from CLOUD_GATEWAY,
//! including authentication token verification.

use crate::command::{Command, CommandType, Door};
use crate::error::ValidationError;

/// Validates commands received from the cloud.
#[derive(Debug, Clone)]
pub struct CommandValidator {
    /// Valid authentication tokens
    valid_tokens: Vec<String>,
}

impl CommandValidator {
    /// Create a new validator with the given valid tokens.
    pub fn new(valid_tokens: Vec<String>) -> Self {
        Self { valid_tokens }
    }

    /// Validate a JSON command payload.
    ///
    /// Returns the parsed Command if valid, or a ValidationError otherwise.
    pub fn validate(&self, json_payload: &[u8]) -> Result<Command, ValidationError> {
        // Parse JSON
        let cmd: Command = self.parse_json(json_payload)?;

        // Validate auth token
        self.validate_auth_token(&cmd.auth_token)?;

        // Validate command type (already validated by serde, but double-check)
        self.validate_command_type(&cmd.command_type)?;

        // Validate doors
        self.validate_doors(&cmd.doors)?;

        // Validate required fields
        if cmd.command_id.is_empty() {
            return Err(ValidationError::MissingField("command_id".to_string()));
        }

        Ok(cmd)
    }

    /// Parse JSON bytes into a Command.
    fn parse_json(&self, json_payload: &[u8]) -> Result<Command, ValidationError> {
        serde_json::from_slice(json_payload).map_err(|e| {
            // Check if it's a missing field error
            let error_msg = e.to_string();
            if error_msg.contains("missing field") {
                // Extract field name from error message
                if let Some(field) = extract_missing_field(&error_msg) {
                    return ValidationError::MissingField(field);
                }
            }
            ValidationError::MalformedJson(error_msg)
        })
    }

    /// Validate the authentication token.
    pub fn validate_auth_token(&self, token: &str) -> Result<(), ValidationError> {
        if token.is_empty() {
            return Err(ValidationError::MissingField("auth_token".to_string()));
        }
        if !self.valid_tokens.contains(&token.to_string()) {
            return Err(ValidationError::AuthFailed);
        }
        Ok(())
    }

    /// Validate the command type.
    pub fn validate_command_type(&self, cmd_type: &CommandType) -> Result<(), ValidationError> {
        // CommandType is already validated by serde deserialization
        // This is a safeguard for programmatic use
        match cmd_type {
            CommandType::Lock | CommandType::Unlock => Ok(()),
        }
    }

    /// Validate the doors list.
    pub fn validate_doors(&self, doors: &[Door]) -> Result<(), ValidationError> {
        if doors.is_empty() {
            return Err(ValidationError::MissingField("doors".to_string()));
        }
        // All door variants are valid (validated by serde)
        Ok(())
    }
}

/// Extract the field name from a serde "missing field" error message.
fn extract_missing_field(error_msg: &str) -> Option<String> {
    // Error format: "missing field `field_name`"
    if let Some(start) = error_msg.find('`') {
        if let Some(end) = error_msg[start + 1..].find('`') {
            return Some(error_msg[start + 1..start + 1 + end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn create_validator() -> CommandValidator {
        CommandValidator::new(vec!["valid-token".to_string(), "demo-token".to_string()])
    }

    #[test]
    fn test_valid_command() {
        let validator = create_validator();
        let json = r#"{
            "command_id": "cmd-123",
            "type": "lock",
            "doors": ["all"],
            "auth_token": "valid-token"
        }"#;

        let result = validator.validate(json.as_bytes());
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.command_id, "cmd-123");
        assert_eq!(cmd.command_type, CommandType::Lock);
    }

    #[test]
    fn test_unlock_command() {
        let validator = create_validator();
        let json = r#"{
            "command_id": "cmd-456",
            "type": "unlock",
            "doors": ["driver"],
            "auth_token": "demo-token"
        }"#;

        let result = validator.validate(json.as_bytes());
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.command_type, CommandType::Unlock);
    }

    #[test]
    fn test_malformed_json() {
        let validator = create_validator();
        let json = b"not valid json {";

        let result = validator.validate(json);
        assert!(matches!(result, Err(ValidationError::MalformedJson(_))));
    }

    #[test]
    fn test_missing_command_id() {
        let validator = create_validator();
        let json = r#"{
            "type": "lock",
            "doors": ["all"],
            "auth_token": "valid-token"
        }"#;

        let result = validator.validate(json.as_bytes());
        assert!(matches!(result, Err(ValidationError::MissingField(ref f)) if f == "command_id"));
    }

    #[test]
    fn test_invalid_auth_token() {
        let validator = create_validator();
        let json = r#"{
            "command_id": "cmd-123",
            "type": "lock",
            "doors": ["all"],
            "auth_token": "invalid-token"
        }"#;

        let result = validator.validate(json.as_bytes());
        assert!(matches!(result, Err(ValidationError::AuthFailed)));
    }

    #[test]
    fn test_invalid_command_type() {
        let validator = create_validator();
        let json = r#"{
            "command_id": "cmd-123",
            "type": "invalid",
            "doors": ["all"],
            "auth_token": "valid-token"
        }"#;

        let result = validator.validate(json.as_bytes());
        // serde will fail to parse invalid command type
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_door() {
        let validator = create_validator();
        let json = r#"{
            "command_id": "cmd-123",
            "type": "lock",
            "doors": ["invalid_door"],
            "auth_token": "valid-token"
        }"#;

        let result = validator.validate(json.as_bytes());
        // serde will fail to parse invalid door
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_doors() {
        let validator = create_validator();
        let json = r#"{
            "command_id": "cmd-123",
            "type": "lock",
            "doors": [],
            "auth_token": "valid-token"
        }"#;

        let result = validator.validate(json.as_bytes());
        assert!(matches!(result, Err(ValidationError::MissingField(ref f)) if f == "doors"));
    }

    // Property 3: Malformed JSON Rejection
    // Validates: Requirements 2.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_malformed_json_rejected(garbage in prop::collection::vec(any::<u8>(), 1..100)) {
            let validator = create_validator();

            // Skip if it accidentally parses as valid JSON
            if serde_json::from_slice::<Command>(&garbage).is_ok() {
                return Ok(());
            }

            let result = validator.validate(&garbage);
            prop_assert!(matches!(
                result,
                Err(ValidationError::MalformedJson(_)) | Err(ValidationError::MissingField(_))
            ));
        }
    }

    // Property 4: Missing Required Fields Rejection
    // Validates: Requirements 2.4
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_missing_command_id_rejected(
            cmd_type in "(lock|unlock)",
            door in "(driver|all)",
            token in "[a-zA-Z0-9-]+"
        ) {
            let validator = CommandValidator::new(vec![token.clone()]);
            let json = format!(r#"{{
                "type": "{}",
                "doors": ["{}"],
                "auth_token": "{}"
            }}"#, cmd_type, door, token);

            let result = validator.validate(json.as_bytes());
            prop_assert!(matches!(result, Err(ValidationError::MissingField(ref f)) if f == "command_id"));
        }

        #[test]
        fn prop_missing_type_rejected(
            cmd_id in "[a-zA-Z0-9-]{1,36}",
            door in "(driver|all)",
            token in "[a-zA-Z0-9-]+"
        ) {
            let validator = CommandValidator::new(vec![token.clone()]);
            let json = format!(r#"{{
                "command_id": "{}",
                "doors": ["{}"],
                "auth_token": "{}"
            }}"#, cmd_id, door, token);

            let result = validator.validate(json.as_bytes());
            prop_assert!(matches!(result, Err(ValidationError::MissingField(ref f)) if f == "type"));
        }
    }

    // Property 5: Invalid Auth Token Rejection
    // Validates: Requirements 3.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_invalid_auth_token_rejected(
            cmd_id in "[a-zA-Z0-9-]{1,36}",
            cmd_type in "(lock|unlock)",
            door in "(driver|all)",
            invalid_token in "[a-zA-Z0-9-]{1,32}"
        ) {
            // Use tokens that are guaranteed different from the invalid_token
            let validator = CommandValidator::new(vec!["VALID_TOKEN_1".to_string(), "VALID_TOKEN_2".to_string()]);

            // Skip if the invalid_token happens to match our valid tokens
            if invalid_token == "VALID_TOKEN_1" || invalid_token == "VALID_TOKEN_2" {
                return Ok(());
            }

            let json = format!(r#"{{
                "command_id": "{}",
                "type": "{}",
                "doors": ["{}"],
                "auth_token": "{}"
            }}"#, cmd_id, cmd_type, door, invalid_token);

            let result = validator.validate(json.as_bytes());
            prop_assert!(matches!(result, Err(ValidationError::AuthFailed)));
        }
    }

    // Property 6: Invalid Command Type Rejection
    // Validates: Requirements 3.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_invalid_command_type_rejected(
            cmd_id in "[a-zA-Z0-9-]{1,36}",
            invalid_type in "[a-z]{1,10}",
            door in "(driver|all)",
            token in "[a-zA-Z0-9-]+"
        ) {
            // Skip valid types
            if invalid_type == "lock" || invalid_type == "unlock" {
                return Ok(());
            }

            let validator = CommandValidator::new(vec![token.clone()]);
            let json = format!(r#"{{
                "command_id": "{}",
                "type": "{}",
                "doors": ["{}"],
                "auth_token": "{}"
            }}"#, cmd_id, invalid_type, door, token);

            let result = validator.validate(json.as_bytes());
            // Invalid type causes JSON parse error
            prop_assert!(result.is_err());
        }
    }

    // Property 7: Invalid Door Rejection
    // Validates: Requirements 3.4
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_invalid_door_rejected(
            cmd_id in "[a-zA-Z0-9-]{1,36}",
            cmd_type in "(lock|unlock)",
            invalid_door in "[a-z]{1,10}",
            token in "[a-zA-Z0-9-]+"
        ) {
            // Skip valid doors
            let valid_doors = ["driver", "passenger", "rear_left", "rear_right", "all"];
            if valid_doors.contains(&invalid_door.as_str()) {
                return Ok(());
            }

            let validator = CommandValidator::new(vec![token.clone()]);
            let json = format!(r#"{{
                "command_id": "{}",
                "type": "{}",
                "doors": ["{}"],
                "auth_token": "{}"
            }}"#, cmd_id, cmd_type, invalid_door, token);

            let result = validator.validate(json.as_bytes());
            // Invalid door causes JSON parse error
            prop_assert!(result.is_err());
        }
    }
}
