//! LOCKING_SERVICE — vehicle door lock/unlock command processor.
//!
//! This service subscribes to lock/unlock command signals from the Eclipse
//! Kuksa DATA_BROKER, validates them, enforces safety constraints (vehicle
//! stationary, door closed), and executes the lock/unlock operation by
//! writing the lock state and a command response back to DATA_BROKER.
//!
//! Communication with DATA_BROKER is exclusively via gRPC over Unix Domain
//! Sockets (UDS) for same-partition isolation.

pub mod command;
pub mod safety;
pub mod service;

#[cfg(test)]
mod tests {
    use crate::command::{
        self, CommandResponse, CommandStatus, LockAction, LockCommand, ParseResult,
    };

    #[test]
    fn placeholder_test() {
        assert!(true, "locking-service skeleton compiles and tests run");
    }

    /// TS-02-7: LOCKING_SERVICE parses command JSON (02-REQ-2.2)
    ///
    /// Verify LOCKING_SERVICE correctly parses command JSON payloads
    /// extracting command_id, action, and doors fields.
    #[test]
    fn test_locking_parses_command_json() {
        let json_input = r#"{
            "command_id": "abc",
            "action": "lock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "VIN12345",
            "timestamp": 1700000000
        }"#;

        let cmd: LockCommand = serde_json::from_str(json_input)
            .expect("should parse valid lock command JSON");
        assert_eq!(cmd.command_id, "abc");
        assert_eq!(cmd.action, LockAction::Lock);
        assert_eq!(cmd.doors, vec!["driver"]);
        assert_eq!(cmd.source, "companion_app");
        assert_eq!(cmd.vin, "VIN12345");
        assert_eq!(cmd.timestamp, 1700000000);
    }

    /// TS-02-7 (variant): Verify unlock action parses correctly
    #[test]
    fn test_locking_parses_unlock_command() {
        let json_input = r#"{
            "command_id": "def",
            "action": "unlock",
            "doors": ["driver"],
            "source": "companion_app",
            "vin": "VIN12345",
            "timestamp": 1700000001
        }"#;

        let cmd: LockCommand = serde_json::from_str(json_input)
            .expect("should parse valid unlock command JSON");
        assert_eq!(cmd.command_id, "def");
        assert_eq!(cmd.action, LockAction::Unlock);
        assert_eq!(cmd.doors, vec!["driver"]);
    }

    /// TS-02-E3: Invalid JSON produces InvalidPayload parse result
    #[test]
    fn test_edge_invalid_json_command_parse() {
        match command::parse_command("not valid json {{{") {
            ParseResult::InvalidPayload => {}
            other => panic!("expected InvalidPayload, got: {:?}", other),
        }
    }

    /// TS-02-E4: Unknown action produces UnknownAction parse result
    #[test]
    fn test_edge_unknown_action_parse() {
        let json = r#"{"command_id": "edge-4", "action": "toggle", "doors": ["driver"], "source": "test", "vin": "VIN12345", "timestamp": 1700000000}"#;
        match command::parse_command(json) {
            ParseResult::UnknownAction { command_id } => {
                assert_eq!(command_id.as_deref(), Some("edge-4"));
            }
            other => panic!("expected UnknownAction, got: {:?}", other),
        }
    }

    /// TS-02-E5: Missing fields produces MissingFields parse result
    #[test]
    fn test_edge_missing_fields_parse() {
        // Missing command_id
        let json = r#"{"action": "lock", "doors": ["driver"], "source": "test", "vin": "VIN12345", "timestamp": 1700000000}"#;
        match command::parse_command(json) {
            ParseResult::MissingFields => {}
            other => panic!("expected MissingFields, got: {:?}", other),
        }
    }

    /// Verify response serialization includes expected fields
    #[test]
    fn test_response_serialization_roundtrip() {
        let resp = CommandResponse::success("round-trip");
        let json = resp.to_json();
        let parsed: CommandResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command_id, "round-trip");
        assert_eq!(parsed.status, CommandStatus::Success);
        assert!(parsed.reason.is_none());
    }

    /// Verify failure response includes reason in serialization
    #[test]
    fn test_failure_response_includes_reason() {
        let resp = CommandResponse::failed("fail-test", "vehicle_moving");
        let json = resp.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
        assert_eq!(parsed["command_id"], "fail-test");
    }
}
