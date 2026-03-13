use serde::Serialize;

/// Command response published to Vehicle.Command.Door.Response
#[derive(Debug, Clone, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: i64,
}

/// Build a success response JSON string.
pub fn success_response(command_id: &str) -> String {
    let response = CommandResponse {
        command_id: command_id.to_string(),
        status: "success".to_string(),
        reason: None,
        timestamp: current_unix_timestamp(),
    };
    serde_json::to_string(&response).expect("failed to serialize success response")
}

/// Build a failure response JSON string.
pub fn failure_response(command_id: &str, reason: &str) -> String {
    let response = CommandResponse {
        command_id: command_id.to_string(),
        status: "failed".to_string(),
        reason: Some(reason.to_string()),
        timestamp: current_unix_timestamp(),
    };
    serde_json::to_string(&response).expect("failed to serialize failure response")
}

/// Get the current Unix timestamp in seconds.
fn current_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-14: Success Response Format
    #[test]
    fn test_success_response_format() {
        let json = success_response("abc-123");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("should be valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "success");
        assert!(parsed["timestamp"].as_i64().unwrap() > 0);
        assert!(parsed.get("reason").is_none() || parsed["reason"].is_null());
    }

    // TS-03-15: Failure Response Format
    #[test]
    fn test_failure_response_format() {
        let json = failure_response("abc-123", "vehicle_moving");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("should be valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
        assert!(parsed["timestamp"].as_i64().unwrap() > 0);
    }

    // TS-03-16: Response Timestamp
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
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("should be valid JSON");
        let ts = parsed["timestamp"].as_i64().unwrap();
        assert!(ts >= before, "timestamp should be >= before");
        assert!(ts <= after, "timestamp should be <= after");
    }
}
