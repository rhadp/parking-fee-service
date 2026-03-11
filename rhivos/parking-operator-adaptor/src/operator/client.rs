use std::time::Duration;

use reqwest::Client;
use tracing::{error, info};

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
    base_url: String,
    client: Client,
}

impl OperatorClient {
    /// Create a new OperatorClient with the given base URL.
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("failed to create HTTP client");
        Self { base_url, client }
    }

    /// Start a parking session via POST /parking/start.
    pub async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        let url = format!("{}/parking/start", self.base_url);
        let timestamp = chrono::Utc::now().timestamp();
        let body = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp,
        };

        info!(%url, %vehicle_id, %zone_id, "sending start_session request");

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    OperatorError::Timeout
                } else {
                    OperatorError::Unreachable(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            error!(http_status = %status, body = %body_text, "operator start_session failed");
            return Err(OperatorError::HttpError {
                status: status.as_u16(),
                body: body_text,
            });
        }

        response
            .json::<StartResponse>()
            .await
            .map_err(|e| OperatorError::ParseError(e.to_string()))
    }

    /// Stop a parking session via POST /parking/stop.
    pub async fn stop_session(
        &self,
        session_id: &str,
    ) -> Result<StopResponse, OperatorError> {
        let url = format!("{}/parking/stop", self.base_url);
        let timestamp = chrono::Utc::now().timestamp();
        let body = StopRequest {
            session_id: session_id.to_string(),
            timestamp,
        };

        info!(%url, %session_id, "sending stop_session request");

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    OperatorError::Timeout
                } else {
                    OperatorError::Unreachable(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            error!(http_status = %status, body = %body_text, "operator stop_session failed");
            return Err(OperatorError::HttpError {
                status: status.as_u16(),
                body: body_text,
            });
        }

        response
            .json::<StopResponse>()
            .await
            .map_err(|e| OperatorError::ParseError(e.to_string()))
    }

    /// Query session status via GET /parking/status/{session_id}.
    pub async fn get_status(
        &self,
        session_id: &str,
    ) -> Result<StatusResponse, OperatorError> {
        let url = format!("{}/parking/status/{}", self.base_url, session_id);

        info!(%url, %session_id, "sending get_status request");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    OperatorError::Timeout
                } else {
                    OperatorError::Unreachable(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            error!(http_status = %status, body = %body_text, "operator get_status failed");
            return Err(OperatorError::HttpError {
                status: status.as_u16(),
                body: body_text,
            });
        }

        response
            .json::<StatusResponse>()
            .await
            .map_err(|e| OperatorError::ParseError(e.to_string()))
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

    /// TS-08-7.E1: Verify OperatorClient handles unreachable operator gracefully.
    #[tokio::test]
    async fn test_start_session_unreachable() {
        let client = OperatorClient::new("http://127.0.0.1:19876".to_string());
        let result = client.start_session("VIN-001", "zone-1").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OperatorError::Unreachable(_) => {} // expected
            other => panic!("expected Unreachable, got: {other:?}"),
        }
    }

    /// Verify OperatorClient handles unreachable operator on stop.
    #[tokio::test]
    async fn test_stop_session_unreachable() {
        let client = OperatorClient::new("http://127.0.0.1:19876".to_string());
        let result = client.stop_session("session-123").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OperatorError::Unreachable(_) => {}
            other => panic!("expected Unreachable, got: {other:?}"),
        }
    }

    /// Verify OperatorClient handles unreachable operator on get_status.
    #[tokio::test]
    async fn test_get_status_unreachable() {
        let client = OperatorClient::new("http://127.0.0.1:19876".to_string());
        let result = client.get_status("session-123").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OperatorError::Unreachable(_) => {}
            other => panic!("expected Unreachable, got: {other:?}"),
        }
    }
}
