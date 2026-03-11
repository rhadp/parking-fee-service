use serde::{Deserialize, Serialize};

/// Request body for POST /parking/start
#[derive(Debug, Serialize, Deserialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub zone_id: String,
    pub timestamp: i64,
}

/// Response body from POST /parking/start
#[derive(Debug, Serialize, Deserialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
}

/// Request body for POST /parking/stop
#[derive(Debug, Serialize, Deserialize)]
pub struct StopRequest {
    pub session_id: String,
    pub timestamp: i64,
}

/// Response body from POST /parking/stop
#[derive(Debug, Serialize, Deserialize)]
pub struct StopResponse {
    pub session_id: String,
    pub duration: i64,
    pub fee: f64,
    pub status: String,
}

/// Response body from GET /parking/status/{session_id}
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub session_id: String,
    pub status: String,
    pub rate_type: String,
    pub rate_amount: f64,
    pub currency: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-08-9: Verify start request serializes correctly.
    #[test]
    fn test_start_request_serialization() {
        let req = StartRequest {
            vehicle_id: "VIN-001".to_string(),
            zone_id: "zone-1".to_string(),
            timestamp: 1700000000,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["vehicle_id"], "VIN-001");
        assert_eq!(parsed["zone_id"], "zone-1");
        assert_eq!(parsed["timestamp"], 1700000000);
    }

    /// TS-08-9: Verify stop request serializes correctly.
    #[test]
    fn test_stop_request_serialization() {
        let req = StopRequest {
            session_id: "session-abc".to_string(),
            timestamp: 1700003600,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["session_id"], "session-abc");
        assert_eq!(parsed["timestamp"], 1700003600);
    }
}
