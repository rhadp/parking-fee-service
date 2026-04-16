//! Command response building.
//!
//! `success_response` and `failure_response` produce the JSON strings that
//! LOCKING_SERVICE publishes to `Vehicle.Command.Door.Response`.

use serde::Serialize;

/// Serialisable representation of a command response.
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: i64,
}

/// Build a success response JSON string for `command_id`.
///
/// The `reason` field is absent in success responses (03-REQ-5.3).
pub fn success_response(_command_id: &str) -> String {
    todo!("Implement success_response in task group 2")
}

/// Build a failure response JSON string for `command_id` with the given `reason`.
///
/// Valid reasons: "vehicle_moving", "door_open", "unsupported_door", "invalid_command".
pub fn failure_response(_command_id: &str, _reason: &str) -> String {
    todo!("Implement failure_response in task group 2")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-14: Success response has command_id, status "success", timestamp, no reason
    #[test]
    fn test_success_response_format() {
        let json = success_response("abc-123");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("response should be valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "success");
        assert!(
            parsed["timestamp"].is_i64() || parsed["timestamp"].is_u64(),
            "timestamp should be a number"
        );
        assert!(
            parsed["timestamp"].as_i64().unwrap_or(0) > 0,
            "timestamp should be positive"
        );
        assert!(
            parsed["reason"].is_null(),
            "success response must not include a reason field"
        );
    }

    // TS-03-15: Failure response has command_id, status "failed", reason, timestamp
    #[test]
    fn test_failure_response_format() {
        let json = failure_response("abc-123", "vehicle_moving");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("response should be valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
        assert!(
            parsed["timestamp"].as_i64().unwrap_or(0) > 0,
            "timestamp should be positive"
        );
    }

    // TS-03-16: Response timestamp is close to current time
    #[test]
    fn test_response_timestamp() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let json = success_response("x");
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("response should be valid JSON");
        let ts = parsed["timestamp"].as_i64().expect("timestamp should be i64");
        assert!(ts >= before, "timestamp should not be before function call");
        assert!(ts <= after, "timestamp should not be after function call");
    }

    // Additional: all four failure reasons are accepted
    #[test]
    fn test_failure_reason_door_open() {
        let json = failure_response("cmd-1", "door_open");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["reason"], "door_open");
        assert_eq!(parsed["status"], "failed");
    }

    #[test]
    fn test_failure_reason_unsupported_door() {
        let json = failure_response("cmd-2", "unsupported_door");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["reason"], "unsupported_door");
    }

    #[test]
    fn test_failure_reason_invalid_command() {
        let json = failure_response("cmd-3", "invalid_command");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["reason"], "invalid_command");
    }
}
