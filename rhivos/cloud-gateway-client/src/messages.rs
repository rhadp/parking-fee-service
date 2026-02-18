//! Shared MQTT message types and topic patterns for vehicle-to-cloud
//! communication.
//!
//! These types are serialized as JSON over MQTT. The schemas defined here must
//! match the Go-side definitions in `backend/cloud-gateway/messages/types.go`
//! to ensure wire-format compatibility.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// MQTT Topic Patterns
// ---------------------------------------------------------------------------

/// Topic for lock/unlock commands sent from CLOUD_GATEWAY to a vehicle (QoS 2).
///
/// Format: `vehicles/{vin}/commands`
pub const TOPIC_COMMANDS: &str = "vehicles/{vin}/commands";

/// Topic for command results sent from vehicle to CLOUD_GATEWAY (QoS 2).
///
/// Format: `vehicles/{vin}/command_responses`
pub const TOPIC_COMMAND_RESPONSES: &str = "vehicles/{vin}/command_responses";

/// Topic for on-demand status requests from CLOUD_GATEWAY (QoS 2).
///
/// Format: `vehicles/{vin}/status_request`
pub const TOPIC_STATUS_REQUEST: &str = "vehicles/{vin}/status_request";

/// Topic for status responses from vehicle to CLOUD_GATEWAY (QoS 2).
///
/// Format: `vehicles/{vin}/status_response`
pub const TOPIC_STATUS_RESPONSE: &str = "vehicles/{vin}/status_response";

/// Topic for periodic telemetry from vehicle to CLOUD_GATEWAY (QoS 0).
///
/// Format: `vehicles/{vin}/telemetry`
pub const TOPIC_TELEMETRY: &str = "vehicles/{vin}/telemetry";

/// Topic for vehicle registration on startup (QoS 2).
///
/// Format: `vehicles/{vin}/registration`
pub const TOPIC_REGISTRATION: &str = "vehicles/{vin}/registration";

/// Returns a fully-qualified MQTT topic by replacing `{vin}` with the given VIN.
pub fn topic_for(pattern: &str, vin: &str) -> String {
    pattern.replace("{vin}", vin)
}

// ---------------------------------------------------------------------------
// Command Types and Results
// ---------------------------------------------------------------------------

/// The type of a lock/unlock command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandType {
    /// Request the vehicle to lock.
    Lock,
    /// Request the vehicle to unlock.
    Unlock,
}

/// The outcome of a lock/unlock command.
///
/// Variant names use SCREAMING_SNAKE_CASE to match the MQTT wire format
/// (e.g. `"SUCCESS"`, `"REJECTED_SPEED"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub enum CommandResult {
    /// The command was executed successfully.
    SUCCESS,
    /// The command was rejected because speed exceeded the safety threshold.
    REJECTED_SPEED,
    /// The command was rejected because a door was open.
    REJECTED_DOOR_OPEN,
}

// ---------------------------------------------------------------------------
// Message Structs
// ---------------------------------------------------------------------------

/// Published by CLOUD_GATEWAY to `vehicles/{vin}/commands`.
///
/// Instructs the vehicle to lock or unlock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandMessage {
    pub command_id: String,
    #[serde(rename = "type")]
    pub command_type: CommandType,
    pub timestamp: i64,
}

/// Published by CLOUD_GATEWAY_CLIENT to `vehicles/{vin}/command_responses`.
///
/// Reports the result of a command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandResponse {
    pub command_id: String,
    #[serde(rename = "type")]
    pub command_type: CommandType,
    pub result: CommandResult,
    pub timestamp: i64,
}

/// Published by CLOUD_GATEWAY to `vehicles/{vin}/status_request`.
///
/// Requests current vehicle state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatusRequest {
    pub request_id: String,
    pub timestamp: i64,
}

/// Published by CLOUD_GATEWAY_CLIENT to `vehicles/{vin}/status_response`.
///
/// Responds with current vehicle state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatusResponse {
    pub request_id: String,
    pub vin: String,
    pub is_locked: Option<bool>,
    pub is_door_open: Option<bool>,
    pub speed: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub parking_session_active: Option<bool>,
    pub timestamp: i64,
}

/// Published periodically by CLOUD_GATEWAY_CLIENT to `vehicles/{vin}/telemetry`.
///
/// Reports current vehicle state. Same shape as [`StatusResponse`] but without
/// `request_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelemetryMessage {
    pub vin: String,
    pub is_locked: Option<bool>,
    pub is_door_open: Option<bool>,
    pub speed: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub parking_session_active: Option<bool>,
    pub timestamp: i64,
}

/// Published by CLOUD_GATEWAY_CLIENT to `vehicles/{vin}/registration` on
/// startup to register the vehicle with CLOUD_GATEWAY.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistrationMessage {
    pub vin: String,
    pub pairing_pin: String,
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_for_replaces_vin() {
        assert_eq!(
            topic_for(TOPIC_COMMANDS, "DEMO0000000000001"),
            "vehicles/DEMO0000000000001/commands"
        );
        assert_eq!(
            topic_for(TOPIC_COMMAND_RESPONSES, "VIN123"),
            "vehicles/VIN123/command_responses"
        );
    }

    #[test]
    fn command_type_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&CommandType::Lock).unwrap(),
            r#""lock""#
        );
        assert_eq!(
            serde_json::to_string(&CommandType::Unlock).unwrap(),
            r#""unlock""#
        );
    }

    #[test]
    fn command_type_deserializes_lowercase() {
        assert_eq!(
            serde_json::from_str::<CommandType>(r#""lock""#).unwrap(),
            CommandType::Lock
        );
        assert_eq!(
            serde_json::from_str::<CommandType>(r#""unlock""#).unwrap(),
            CommandType::Unlock
        );
    }

    #[test]
    fn command_result_serializes_uppercase() {
        assert_eq!(
            serde_json::to_string(&CommandResult::SUCCESS).unwrap(),
            r#""SUCCESS""#
        );
        assert_eq!(
            serde_json::to_string(&CommandResult::REJECTED_SPEED).unwrap(),
            r#""REJECTED_SPEED""#
        );
        assert_eq!(
            serde_json::to_string(&CommandResult::REJECTED_DOOR_OPEN).unwrap(),
            r#""REJECTED_DOOR_OPEN""#
        );
    }

    #[test]
    fn command_result_deserializes_uppercase() {
        assert_eq!(
            serde_json::from_str::<CommandResult>(r#""SUCCESS""#).unwrap(),
            CommandResult::SUCCESS
        );
        assert_eq!(
            serde_json::from_str::<CommandResult>(r#""REJECTED_SPEED""#).unwrap(),
            CommandResult::REJECTED_SPEED
        );
    }

    // -----------------------------------------------------------------------
    // Schema compatibility tests — these produce the same JSON as the Go tests
    // in backend/cloud-gateway/messages/types_test.go to verify wire-format
    // agreement between Go and Rust.
    // -----------------------------------------------------------------------

    #[test]
    fn command_message_json_matches_go() {
        let msg = CommandMessage {
            command_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            command_type: CommandType::Lock,
            timestamp: 1708300800,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["command_id"], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(json["type"], "lock");
        assert_eq!(json["timestamp"], 1708300800);
        // Verify no extra fields.
        assert_eq!(json.as_object().unwrap().len(), 3);
    }

    #[test]
    fn command_response_json_matches_go() {
        let msg = CommandResponse {
            command_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            command_type: CommandType::Lock,
            result: CommandResult::SUCCESS,
            timestamp: 1708300801,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["command_id"], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(json["type"], "lock");
        assert_eq!(json["result"], "SUCCESS");
        assert_eq!(json["timestamp"], 1708300801);
        assert_eq!(json.as_object().unwrap().len(), 4);
    }

    #[test]
    fn status_request_json_matches_go() {
        let msg = StatusRequest {
            request_id: "660e8400-e29b-41d4-a716-446655440000".to_string(),
            timestamp: 1708300802,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["request_id"], "660e8400-e29b-41d4-a716-446655440000");
        assert_eq!(json["timestamp"], 1708300802);
        assert_eq!(json.as_object().unwrap().len(), 2);
    }

    #[test]
    fn status_response_json_matches_go() {
        let locked = true;
        let door_open = false;
        let speed = 0.0_f64;
        let lat = 48.1351_f64;
        let lon = 11.582_f64;
        let parking = false;

        let msg = StatusResponse {
            request_id: "660e8400-e29b-41d4-a716-446655440000".to_string(),
            vin: "DEMO0000000000001".to_string(),
            is_locked: Some(locked),
            is_door_open: Some(door_open),
            speed: Some(speed),
            latitude: Some(lat),
            longitude: Some(lon),
            parking_session_active: Some(parking),
            timestamp: 1708300802,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["request_id"], "660e8400-e29b-41d4-a716-446655440000");
        assert_eq!(json["vin"], "DEMO0000000000001");
        assert_eq!(json["is_locked"], true);
        assert_eq!(json["is_door_open"], false);
        assert_eq!(json["speed"], 0.0);
        assert_eq!(json["latitude"], 48.1351);
        assert_eq!(json["longitude"], 11.582);
        assert_eq!(json["parking_session_active"], false);
        assert_eq!(json["timestamp"], 1708300802);
        assert_eq!(json.as_object().unwrap().len(), 9);
    }

    #[test]
    fn telemetry_message_json_matches_go() {
        let locked = true;
        let door_open = false;
        let speed = 0.0_f64;
        let lat = 48.1351_f64;
        let lon = 11.582_f64;
        let parking = false;

        let msg = TelemetryMessage {
            vin: "DEMO0000000000001".to_string(),
            is_locked: Some(locked),
            is_door_open: Some(door_open),
            speed: Some(speed),
            latitude: Some(lat),
            longitude: Some(lon),
            parking_session_active: Some(parking),
            timestamp: 1708300802,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["vin"], "DEMO0000000000001");
        assert_eq!(json["is_locked"], true);
        assert_eq!(json["is_door_open"], false);
        assert_eq!(json["speed"], 0.0);
        assert_eq!(json["latitude"], 48.1351);
        assert_eq!(json["longitude"], 11.582);
        assert_eq!(json["parking_session_active"], false);
        assert_eq!(json["timestamp"], 1708300802);
        // No request_id in telemetry.
        assert!(json.get("request_id").is_none());
        assert_eq!(json.as_object().unwrap().len(), 8);
    }

    #[test]
    fn registration_message_json_matches_go() {
        let msg = RegistrationMessage {
            vin: "DEMO0000000000001".to_string(),
            pairing_pin: "482916".to_string(),
            timestamp: 1708300800,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["vin"], "DEMO0000000000001");
        assert_eq!(json["pairing_pin"], "482916");
        assert_eq!(json["timestamp"], 1708300800);
        assert_eq!(json.as_object().unwrap().len(), 3);
    }

    #[test]
    fn status_response_null_fields() {
        let msg = StatusResponse {
            request_id: "test-id".to_string(),
            vin: "VIN123".to_string(),
            is_locked: None,
            is_door_open: None,
            speed: None,
            latitude: None,
            longitude: None,
            parking_session_active: None,
            timestamp: 1708300802,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert!(json["is_locked"].is_null());
        assert!(json["is_door_open"].is_null());
        assert!(json["speed"].is_null());
        assert!(json["latitude"].is_null());
        assert!(json["longitude"].is_null());
        assert!(json["parking_session_active"].is_null());
    }

    #[test]
    fn telemetry_message_null_fields() {
        let msg = TelemetryMessage {
            vin: "VIN123".to_string(),
            is_locked: None,
            is_door_open: None,
            speed: None,
            latitude: None,
            longitude: None,
            parking_session_active: None,
            timestamp: 1708300802,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert!(json["is_locked"].is_null());
        assert!(json["speed"].is_null());
    }

    #[test]
    fn command_message_roundtrip() {
        let msg = CommandMessage {
            command_id: "abc-123".to_string(),
            command_type: CommandType::Unlock,
            timestamp: 12345,
        };
        let json_str = serde_json::to_string(&msg).unwrap();
        let deserialized: CommandMessage = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn command_response_roundtrip() {
        let msg = CommandResponse {
            command_id: "abc-123".to_string(),
            command_type: CommandType::Lock,
            result: CommandResult::REJECTED_DOOR_OPEN,
            timestamp: 12345,
        };
        let json_str = serde_json::to_string(&msg).unwrap();
        let deserialized: CommandResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn invalid_command_type_rejected() {
        let result = serde_json::from_str::<CommandMessage>(
            r#"{"command_id":"x","type":"invalid","timestamp":0}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn invalid_command_result_rejected() {
        let result = serde_json::from_str::<CommandResponse>(
            r#"{"command_id":"x","type":"lock","result":"UNKNOWN","timestamp":0}"#,
        );
        assert!(result.is_err());
    }
}
