//! Command response building and serialization.
use serde::Serialize;

/// A JSON-serializable command response.
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: i64,
}

/// Build a success response JSON string for the given command_id.
pub fn success_response(_command_id: &str) -> String {
    todo!("implemented in task group 2")
}

/// Build a failure response JSON string with the given reason.
pub fn failure_response(_command_id: &str, _reason: &str) -> String {
    todo!("implemented in task group 2")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-14: Success response format — command_id, status "success", timestamp, no reason
    #[test]
    fn test_success_response_format() {
        let json = success_response("abc-123");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("success_response must be valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "success");
        assert!(parsed["timestamp"].is_number(), "timestamp must be a number");
        assert!(
            parsed["reason"].is_null(),
            "success response must not include reason"
        );
    }

    // TS-03-15: Failure response format — command_id, status "failed", reason, timestamp
    #[test]
    fn test_failure_response_format() {
        let json = failure_response("abc-123", "vehicle_moving");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("failure_response must be valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
        assert!(parsed["timestamp"].is_number(), "timestamp must be a number");
    }

    // TS-03-16: Timestamp is between before and after current time
    #[test]
    fn test_response_timestamp() {
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let json = success_response("x");
        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let ts = parsed["timestamp"].as_i64().expect("timestamp must be i64");
        assert!(ts >= before, "timestamp {ts} < before {before}");
        assert!(ts <= after, "timestamp {ts} > after {after}");
    }

    // All failure reasons must be serialized correctly
    #[test]
    fn test_failure_response_door_open_reason() {
        let json = failure_response("id-1", "door_open");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["reason"], "door_open");
        assert_eq!(parsed["status"], "failed");
    }

    #[test]
    fn test_failure_response_unsupported_door_reason() {
        let json = failure_response("id-2", "unsupported_door");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["reason"], "unsupported_door");
    }

    #[test]
    fn test_failure_response_invalid_command_reason() {
        let json = failure_response("id-3", "invalid_command");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["reason"], "invalid_command");
    }
}
