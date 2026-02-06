//! Command and response types for cloud-gateway-client.
//!
//! This module defines the JSON message formats for commands received
//! from CLOUD_GATEWAY and responses published back.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// A lock/unlock command received from the cloud.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    /// Unique identifier for tracking command execution
    pub command_id: String,

    /// The type of command (lock or unlock)
    #[serde(rename = "type")]
    pub command_type: CommandType,

    /// Which doors to operate on
    pub doors: Vec<Door>,

    /// Authentication token for command authorization
    pub auth_token: String,

    /// Timestamp from CLOUD_GATEWAY (optional, for audit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Command type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandType {
    /// Lock the specified doors
    Lock,
    /// Unlock the specified doors
    Unlock,
}

/// Door identifier enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Door {
    /// Driver's door
    Driver,
    /// Front passenger door
    Passenger,
    /// Rear left door
    RearLeft,
    /// Rear right door
    RearRight,
    /// All doors
    All,
}

/// Response to a command, published back to the cloud.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandResponse {
    /// The command_id from the original command
    pub command_id: String,

    /// Whether the command succeeded or failed
    pub status: ResponseStatus,

    /// Error code (present when status is Failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

    /// Human-readable error message (present when status is Failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// ISO8601 timestamp for audit logging
    pub timestamp: String,
}

/// Response status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    /// Command executed successfully
    Success,
    /// Command failed
    Failed,
}

impl CommandResponse {
    /// Create a success response for the given command_id.
    pub fn success(command_id: impl Into<String>) -> Self {
        Self {
            command_id: command_id.into(),
            status: ResponseStatus::Success,
            error_code: None,
            error_message: None,
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Create a failure response with error details.
    pub fn failure(
        command_id: impl Into<String>,
        error_code: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        Self {
            command_id: command_id.into(),
            status: ResponseStatus::Failed,
            error_code: Some(error_code.into()),
            error_message: Some(error_message.into()),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

impl Door {
    /// Convert to proto Door enum value.
    pub fn to_proto(&self) -> i32 {
        match self {
            Door::Driver => 1,
            Door::Passenger => 2,
            Door::RearLeft => 3,
            Door::RearRight => 4,
            Door::All => 5,
        }
    }

    /// Try to convert from a string representation.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "driver" => Some(Door::Driver),
            "passenger" => Some(Door::Passenger),
            "rear_left" => Some(Door::RearLeft),
            "rear_right" => Some(Door::RearRight),
            "all" => Some(Door::All),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_command_serialization() {
        let cmd = Command {
            command_id: "test-123".to_string(),
            command_type: CommandType::Lock,
            doors: vec![Door::All],
            auth_token: "token".to_string(),
            timestamp: None,
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"lock\""));
        assert!(json.contains("\"doors\":[\"all\"]"));

        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command_id, cmd.command_id);
        assert_eq!(parsed.command_type, cmd.command_type);
    }

    #[test]
    fn test_response_success() {
        let resp = CommandResponse::success("cmd-1");
        assert_eq!(resp.status, ResponseStatus::Success);
        assert!(resp.error_code.is_none());
        assert!(resp.error_message.is_none());
    }

    #[test]
    fn test_response_failure() {
        let resp = CommandResponse::failure("cmd-1", "AUTH_FAILED", "Invalid token");
        assert_eq!(resp.status, ResponseStatus::Failed);
        assert_eq!(resp.error_code.as_deref(), Some("AUTH_FAILED"));
        assert_eq!(resp.error_message.as_deref(), Some("Invalid token"));
    }

    // Property test strategies
    fn arb_command_type() -> impl Strategy<Value = CommandType> {
        prop_oneof![Just(CommandType::Lock), Just(CommandType::Unlock),]
    }

    fn arb_door() -> impl Strategy<Value = Door> {
        prop_oneof![
            Just(Door::Driver),
            Just(Door::Passenger),
            Just(Door::RearLeft),
            Just(Door::RearRight),
            Just(Door::All),
        ]
    }

    fn arb_doors() -> impl Strategy<Value = Vec<Door>> {
        prop::collection::vec(arb_door(), 1..5)
    }

    fn arb_command() -> impl Strategy<Value = Command> {
        (
            "[a-zA-Z0-9-]{1,36}",
            arb_command_type(),
            arb_doors(),
            "[a-zA-Z0-9-]{1,64}",
        )
            .prop_map(|(command_id, command_type, doors, auth_token)| Command {
                command_id,
                command_type,
                doors,
                auth_token,
                timestamp: None,
            })
    }

    // Property 2: Command JSON Round-Trip
    // Validates: Requirements 2.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_command_json_round_trip(cmd in arb_command()) {
            // Serialize to JSON
            let json = serde_json::to_string(&cmd).unwrap();

            // Deserialize back
            let parsed: Command = serde_json::from_str(&json).unwrap();

            // Verify equivalence
            prop_assert_eq!(parsed.command_id, cmd.command_id);
            prop_assert_eq!(parsed.command_type, cmd.command_type);
            prop_assert_eq!(parsed.doors, cmd.doors);
            prop_assert_eq!(parsed.auth_token, cmd.auth_token);
        }
    }
}
