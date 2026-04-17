//! Command response serialization.
//!
//! Builds the JSON payloads published to `Vehicle.Command.Door.Response`
//! (03-REQ-5.1, 03-REQ-5.2, 03-REQ-5.3).

#![allow(dead_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX epoch")
        .as_secs() as i64
}

// ── CommandResponse ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    /// Only present on failure (03-REQ-5.3: success response must NOT include reason).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: i64,
}

// ── Builder functions ─────────────────────────────────────────────────────────

/// Build a JSON success response for the given command_id (03-REQ-5.1).
pub fn success_response(command_id: &str) -> String {
    let resp = CommandResponse {
        command_id: command_id.to_string(),
        status: "success".to_string(),
        reason: None,
        timestamp: current_timestamp(),
    };
    serde_json::to_string(&resp).expect("CommandResponse is always serializable")
}

/// Build a JSON failure response for the given command_id and reason (03-REQ-5.2).
pub fn failure_response(command_id: &str, reason: &str) -> String {
    let resp = CommandResponse {
        command_id: command_id.to_string(),
        status: "failed".to_string(),
        reason: Some(reason.to_string()),
        timestamp: current_timestamp(),
    };
    serde_json::to_string(&resp).expect("CommandResponse is always serializable")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// TS-03-14 / 03-REQ-5.1 / 03-REQ-5.3: success JSON has required fields, no reason.
    #[test]
    fn test_success_response_format() {
        let json = success_response("abc-123");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("success_response must produce valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "success");
        assert!(
            parsed["timestamp"].as_i64().unwrap_or(0) > 0,
            "timestamp must be positive"
        );
        // 03-REQ-5.3: reason field must be absent on success.
        assert!(
            parsed.get("reason").map_or(true, |v| v.is_null()),
            "success response must not include a reason field"
        );
    }

    /// TS-03-15 / 03-REQ-5.2: failure JSON has command_id, status, reason, timestamp.
    #[test]
    fn test_failure_response_format() {
        let json = failure_response("abc-123", "vehicle_moving");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("failure_response must produce valid JSON");
        assert_eq!(parsed["command_id"], "abc-123");
        assert_eq!(parsed["status"], "failed");
        assert_eq!(parsed["reason"], "vehicle_moving");
        assert!(
            parsed["timestamp"].as_i64().unwrap_or(0) > 0,
            "timestamp must be positive"
        );
    }

    /// TS-03-16 / 03-REQ-5.1: timestamp falls between before/after recording time.
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
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("must produce valid JSON");
        let ts = parsed["timestamp"].as_i64().expect("timestamp must be an integer");
        assert!(ts >= before, "timestamp must be >= time before call");
        assert!(ts <= after, "timestamp must be <= time after call");
    }

    /// All failure reasons produce a parseable response with correct fields.
    #[test]
    fn test_all_failure_reasons() {
        let reasons = ["vehicle_moving", "door_open", "unsupported_door", "invalid_command"];
        for reason in &reasons {
            let json = failure_response("cmd-1", reason);
            let parsed: serde_json::Value =
                serde_json::from_str(&json).unwrap_or_else(|_| panic!("invalid JSON for reason {reason}"));
            assert_eq!(parsed["status"], "failed");
            assert_eq!(parsed["reason"].as_str().unwrap(), *reason);
        }
    }
}
