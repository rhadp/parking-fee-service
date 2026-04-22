use serde::Serialize;

/// A command response to publish to DATA_BROKER.
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    /// Echoed command_id from request.
    pub command_id: String,
    /// "success" or "failed".
    pub status: String,
    /// Present only on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Current Unix timestamp in seconds.
    pub timestamp: i64,
}

/// Build a success response JSON string.
pub fn success_response(command_id: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs() as i64;
    let resp = CommandResponse {
        command_id: command_id.to_string(),
        status: "success".to_string(),
        reason: None,
        timestamp,
    };
    serde_json::to_string(&resp).expect("CommandResponse serialization should not fail")
}

/// Build a failure response JSON string.
pub fn failure_response(command_id: &str, reason: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs() as i64;
    let resp = CommandResponse {
        command_id: command_id.to_string(),
        status: "failed".to_string(),
        reason: Some(reason.to_string()),
        timestamp,
    };
    serde_json::to_string(&resp).expect("CommandResponse serialization should not fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    // TS-03-14: Verify the success response JSON contains command_id, status "success",
    // timestamp, and no reason field.
    #[test]
    fn test_success_response_format() {
        let json = success_response("abc-123");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "success");
        assert!(parsed["timestamp"].as_i64().unwrap() > 0);
        assert!(
            parsed.get("reason").is_none() || parsed["reason"].is_null(),
            "success response must not include a reason field"
        );
    }

    // TS-03-15: Verify the failure response JSON contains command_id, status "failed",
    // reason, and timestamp.
    #[test]
    fn test_failure_response_format() {
        let json = failure_response("abc-123", "vehicle_moving");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
        assert!(parsed["timestamp"].as_i64().unwrap() > 0);
    }

    // TS-03-16: Verify the response timestamp is a valid Unix timestamp close to
    // current time.
    #[test]
    fn test_response_timestamp() {
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let json = success_response("x");
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        let ts = parsed["timestamp"].as_i64().unwrap();
        assert!(ts >= before, "timestamp {ts} should be >= {before}");
        assert!(ts <= after, "timestamp {ts} should be <= {after}");
    }
}
