use serde::{Deserialize, Serialize};

/// Request body for POST /parking/start.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub zone_id: String,
    pub timestamp: i64,
}

/// Response body from POST /parking/start.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
}

/// Request body for POST /parking/stop.
#[derive(Debug, Serialize, Deserialize)]
pub struct StopRequest {
    pub session_id: String,
    pub timestamp: i64,
}

/// Response body from POST /parking/stop.
#[derive(Debug, Serialize, Deserialize)]
pub struct StopResponse {
    pub session_id: String,
    pub duration: i64,
    pub fee: f64,
    pub status: String,
}

/// Response body from GET /parking/status/{session_id}.
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

    /// TS-08-9: Verify start request serialization format.
    #[test]
    fn test_start_session_request_format() {
        let req = StartRequest {
            vehicle_id: "VIN-001".to_string(),
            zone_id: "zone-1".to_string(),
            timestamp: 1700000000,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["vehicle_id"], "VIN-001");
        assert_eq!(json["zone_id"], "zone-1");
        assert_eq!(json["timestamp"], 1700000000);
    }

    /// TS-08-9: Verify stop request serialization format.
    #[test]
    fn test_stop_session_request_format() {
        let req = StopRequest {
            session_id: "sess-abc".to_string(),
            timestamp: 1700003600,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["session_id"], "sess-abc");
        assert_eq!(json["timestamp"], 1700003600);
    }

    /// TS-08-9: Verify start response deserialization.
    #[test]
    fn test_start_session_response_parse() {
        let json_str = r#"{"session_id": "sess-123", "status": "active"}"#;
        let resp: StartResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(resp.session_id, "sess-123");
        assert_eq!(resp.status, "active");
    }

    /// TS-08-9: Verify stop response deserialization.
    #[test]
    fn test_stop_session_response_parse() {
        let json_str = r#"{"session_id": "sess-123", "duration": 3600, "fee": 5.50, "status": "completed"}"#;
        let resp: StopResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(resp.session_id, "sess-123");
        assert_eq!(resp.duration, 3600);
        assert!((resp.fee - 5.50).abs() < f64::EPSILON);
        assert_eq!(resp.status, "completed");
    }

    /// TS-08-10: Verify status query response deserialization.
    #[test]
    fn test_status_query_response_parse() {
        let json_str = r#"{"session_id": "sess-123", "status": "active", "rate_type": "per_hour", "rate_amount": 2.50, "currency": "EUR"}"#;
        let resp: StatusResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(resp.session_id, "sess-123");
        assert_eq!(resp.status, "active");
        assert_eq!(resp.rate_type, "per_hour");
        assert!((resp.rate_amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "EUR");
    }
}
