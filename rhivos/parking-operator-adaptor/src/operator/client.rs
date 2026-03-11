use super::models::*;

/// Error types for operator REST client operations.
#[derive(Debug, thiserror::Error)]
pub enum OperatorError {
    #[error("operator unreachable: {0}")]
    Unreachable(String),
    #[error("request timeout")]
    Timeout,
    #[error("HTTP error {status}: {body}")]
    HttpError { status: u16, body: String },
    #[error("parse error: {0}")]
    ParseError(String),
}

/// REST client for communicating with the PARKING_OPERATOR.
pub struct OperatorClient {
    _base_url: String,
}

impl OperatorClient {
    /// Create a new OperatorClient with the given base URL.
    pub fn new(base_url: String) -> Self {
        Self {
            _base_url: base_url,
        }
    }

    /// Start a parking session via POST /parking/start.
    pub async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        // Stub: will be implemented in task group 4
        todo!("OperatorClient::start_session not yet implemented")
    }

    /// Stop a parking session via POST /parking/stop.
    pub async fn stop_session(
        &self,
        _session_id: &str,
    ) -> Result<StopResponse, OperatorError> {
        // Stub: will be implemented in task group 4
        todo!("OperatorClient::stop_session not yet implemented")
    }

    /// Query session status via GET /parking/status/{session_id}.
    pub async fn get_status(
        &self,
        _session_id: &str,
    ) -> Result<StatusResponse, OperatorError> {
        // Stub: will be implemented in task group 4
        todo!("OperatorClient::get_status not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-08-9: Verify start request body format matches the API contract.
    #[test]
    fn test_start_session_request_format() {
        let req = StartRequest {
            vehicle_id: "VIN-001".to_string(),
            zone_id: "zone-1".to_string(),
            timestamp: 1700000000,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("vehicle_id").is_some(), "must contain vehicle_id");
        assert!(json.get("zone_id").is_some(), "must contain zone_id");
        assert!(json.get("timestamp").is_some(), "must contain timestamp");
    }

    /// TS-08-9: Verify stop request body format matches the API contract.
    #[test]
    fn test_stop_session_request_format() {
        let req = StopRequest {
            session_id: "session-123".to_string(),
            timestamp: 1700003600,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("session_id").is_some(), "must contain session_id");
        assert!(json.get("timestamp").is_some(), "must contain timestamp");
    }

    /// TS-08-9: Verify start response can be deserialized from expected JSON.
    #[test]
    fn test_start_session_response_parse() {
        let json = r#"{"session_id": "abc-123", "status": "active"}"#;
        let resp: StartResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.session_id, "abc-123");
        assert_eq!(resp.status, "active");
    }

    /// TS-08-9: Verify stop response can be deserialized from expected JSON.
    #[test]
    fn test_stop_session_response_parse() {
        let json = r#"{"session_id": "abc-123", "duration": 3600, "fee": 5.50, "status": "completed"}"#;
        let resp: StopResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.session_id, "abc-123");
        assert_eq!(resp.duration, 3600);
        assert_eq!(resp.fee, 5.50);
        assert_eq!(resp.status, "completed");
    }

    /// TS-08-10: Verify status response can be deserialized from expected JSON.
    #[test]
    fn test_status_query_response_parse() {
        let json = r#"{
            "session_id": "abc-123",
            "status": "active",
            "rate_type": "per_hour",
            "rate_amount": 2.50,
            "currency": "EUR"
        }"#;
        let resp: StatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.session_id, "abc-123");
        assert_eq!(resp.status, "active");
        assert_eq!(resp.rate_type, "per_hour");
        assert_eq!(resp.rate_amount, 2.50);
        assert_eq!(resp.currency, "EUR");
    }
}
