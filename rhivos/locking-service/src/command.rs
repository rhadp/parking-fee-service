//! Command parsing and validation for the LOCKING_SERVICE.
//!
//! Handles deserialization and validation of lock/unlock command JSON payloads
//! received via Vehicle.Command.Door.Lock, and serialization of response
//! payloads written to Vehicle.Command.Door.Response.

use serde::{Deserialize, Serialize};

/// A lock/unlock command received from the cloud gateway.
///
/// The command is transmitted as a JSON string via the
/// `Vehicle.Command.Door.Lock` VSS signal.
#[derive(Debug, Deserialize, PartialEq)]
pub struct LockCommand {
    /// Unique identifier for this command, used to correlate with responses.
    pub command_id: String,
    /// The requested action: lock or unlock.
    pub action: LockAction,
    /// The doors to act on (e.g., `["driver"]`).
    #[serde(default)]
    pub doors: Vec<String>,
    /// The source of the command (e.g., `"companion_app"`).
    #[serde(default)]
    pub source: String,
    /// Vehicle identification number.
    #[serde(default)]
    pub vin: String,
    /// Unix timestamp of when the command was issued.
    #[serde(default)]
    pub timestamp: i64,
}

/// The action requested in a lock command.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LockAction {
    /// Lock the specified doors.
    Lock,
    /// Unlock the specified doors.
    Unlock,
}

/// The response to a lock/unlock command.
///
/// Written as a JSON string to `Vehicle.Command.Door.Response`.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CommandResponse {
    /// The command_id of the original command.
    pub command_id: String,
    /// The result status of the command.
    pub status: CommandStatus,
    /// The reason for failure, if status is "failed".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Unix timestamp of when the response was generated.
    pub timestamp: i64,
}

/// The status of a command response.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CommandStatus {
    /// The command was executed successfully.
    Success,
    /// The command failed (see reason field).
    Failed,
}

/// Failure reasons for lock/unlock commands.
pub mod reason {
    /// The JSON payload could not be parsed.
    pub const INVALID_PAYLOAD: &str = "invalid_payload";
    /// The action field is not "lock" or "unlock".
    pub const UNKNOWN_ACTION: &str = "unknown_action";
    /// Required fields (command_id, action) are missing.
    pub const MISSING_FIELDS: &str = "missing_fields";
    /// Vehicle is moving (speed > 0).
    pub const VEHICLE_MOVING: &str = "vehicle_moving";
    /// Door is open (IsOpen == true).
    pub const DOOR_OPEN: &str = "door_open";
}

/// Result of attempting to parse a lock command from a JSON string.
#[derive(Debug)]
pub enum ParseResult {
    /// Successfully parsed a valid command.
    Ok(LockCommand),
    /// The payload was not valid JSON.
    InvalidPayload,
    /// The JSON is valid but required fields are missing.
    MissingFields,
    /// The action field contains an unrecognized value.
    UnknownAction {
        /// The command_id from the payload, if present.
        command_id: Option<String>,
    },
}

/// Parse a JSON string into a `LockCommand`, with detailed error classification.
///
/// Returns a `ParseResult` that distinguishes between:
/// - Valid commands
/// - Invalid JSON (not parseable at all)
/// - Missing required fields (command_id or action)
/// - Unknown action values (not "lock" or "unlock")
pub fn parse_command(json: &str) -> ParseResult {
    // First, try parsing as a generic JSON value to distinguish error types
    let value: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return ParseResult::InvalidPayload,
    };

    // Check for required fields
    let obj = match value.as_object() {
        Some(o) => o,
        None => return ParseResult::MissingFields,
    };

    // command_id is required
    if !obj.contains_key("command_id") {
        return ParseResult::MissingFields;
    }

    // action is required
    if !obj.contains_key("action") {
        return ParseResult::MissingFields;
    }

    let command_id = obj
        .get("command_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Check if action is a known value
    if let Some(action_str) = obj.get("action").and_then(|v| v.as_str()) {
        if action_str != "lock" && action_str != "unlock" {
            return ParseResult::UnknownAction { command_id };
        }
    } else {
        // action is not a string (e.g., a number) — treat as unknown action
        return ParseResult::UnknownAction { command_id };
    }

    // Now try full deserialization
    match serde_json::from_value::<LockCommand>(value) {
        Ok(cmd) => ParseResult::Ok(cmd),
        Err(_) => ParseResult::MissingFields,
    }
}

impl CommandResponse {
    /// Create a success response for a given command_id.
    pub fn success(command_id: &str) -> Self {
        Self {
            command_id: command_id.to_string(),
            status: CommandStatus::Success,
            reason: None,
            timestamp: current_timestamp(),
        }
    }

    /// Create a failure response for a given command_id with a reason.
    pub fn failed(command_id: &str, reason: &str) -> Self {
        Self {
            command_id: command_id.to_string(),
            status: CommandStatus::Failed,
            reason: Some(reason.to_string()),
            timestamp: current_timestamp(),
        }
    }

    /// Create a failure response without a command_id (for parse failures).
    pub fn failed_no_id(reason: &str) -> Self {
        Self {
            command_id: String::new(),
            status: CommandStatus::Failed,
            reason: Some(reason.to_string()),
            timestamp: current_timestamp(),
        }
    }

    /// Serialize the response to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("CommandResponse should always serialize")
    }
}

/// Get the current Unix timestamp in seconds.
fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_lock_command() {
        let json = r#"{
            "command_id": "abc",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "VIN12345",
            "timestamp": 1700000000
        }"#;

        match parse_command(json) {
            ParseResult::Ok(cmd) => {
                assert_eq!(cmd.command_id, "abc");
                assert_eq!(cmd.action, LockAction::Lock);
                assert_eq!(cmd.doors, vec!["driver"]);
                assert_eq!(cmd.source, "companion_app");
                assert_eq!(cmd.vin, "VIN12345");
                assert_eq!(cmd.timestamp, 1700000000);
            }
            other => panic!("expected Ok, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_valid_unlock_command() {
        let json = r#"{
            "command_id": "def",
            "action": "unlock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "VIN12345",
            "timestamp": 1700000001
        }"#;

        match parse_command(json) {
            ParseResult::Ok(cmd) => {
                assert_eq!(cmd.command_id, "def");
                assert_eq!(cmd.action, LockAction::Unlock);
            }
            other => panic!("expected Ok, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_minimal_lock_command() {
        let json = r#"{"command_id": "min", "action": "lock"}"#;

        match parse_command(json) {
            ParseResult::Ok(cmd) => {
                assert_eq!(cmd.command_id, "min");
                assert_eq!(cmd.action, LockAction::Lock);
                assert!(cmd.doors.is_empty());
            }
            other => panic!("expected Ok, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "not valid json {{{";
        match parse_command(json) {
            ParseResult::InvalidPayload => {}
            other => panic!("expected InvalidPayload, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_missing_command_id() {
        let json = r#"{"action": "lock", "doors": ["driver"]}"#;
        match parse_command(json) {
            ParseResult::MissingFields => {}
            other => panic!("expected MissingFields, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_missing_action() {
        let json = r#"{"command_id": "abc", "doors": ["driver"]}"#;
        match parse_command(json) {
            ParseResult::MissingFields => {}
            other => panic!("expected MissingFields, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_unknown_action() {
        let json = r#"{"command_id": "edge-4", "action": "toggle", "doors": ["driver"]}"#;
        match parse_command(json) {
            ParseResult::UnknownAction { command_id } => {
                assert_eq!(command_id.as_deref(), Some("edge-4"));
            }
            other => panic!("expected UnknownAction, got: {:?}", other),
        }
    }

    #[test]
    fn test_response_success() {
        let resp = CommandResponse::success("test-id");
        assert_eq!(resp.command_id, "test-id");
        assert_eq!(resp.status, CommandStatus::Success);
        assert!(resp.reason.is_none());
    }

    #[test]
    fn test_response_failed() {
        let resp = CommandResponse::failed("test-id", reason::VEHICLE_MOVING);
        assert_eq!(resp.command_id, "test-id");
        assert_eq!(resp.status, CommandStatus::Failed);
        assert_eq!(resp.reason.as_deref(), Some("vehicle_moving"));
    }

    #[test]
    fn test_response_serialization() {
        let resp = CommandResponse::success("abc");
        let json = resp.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["command_id"], "abc");
        assert_eq!(parsed["status"], "success");
        // reason should be absent for success
        assert!(parsed.get("reason").is_none());
    }

    #[test]
    fn test_response_failed_serialization() {
        let resp = CommandResponse::failed("xyz", reason::DOOR_OPEN);
        let json = resp.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["command_id"], "xyz");
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "door_open");
    }

    #[test]
    fn test_lock_action_serde() {
        // Test that LockAction serializes to lowercase
        let lock_json = serde_json::to_string(&LockAction::Lock).unwrap();
        assert_eq!(lock_json, r#""lock""#);
        let unlock_json = serde_json::to_string(&LockAction::Unlock).unwrap();
        assert_eq!(unlock_json, r#""unlock""#);
    }
}
