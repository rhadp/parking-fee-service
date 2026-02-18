//! REST client for the PARKING_OPERATOR service.
//!
//! [`OperatorClient`] communicates with the PARKING_OPERATOR's REST API to
//! start/stop parking sessions and query rates. Request and response types
//! are defined as serde-serializable structs matching the PARKING_OPERATOR's
//! JSON schema.
//!
//! # Requirements
//!
//! - 04-REQ-1.2: Call `POST /parking/start` on lock events.
//! - 04-REQ-1.4: Call `POST /parking/stop` on unlock events.
//! - 04-REQ-2.5: Query `GET /parking/rate` for rate info.
//! - 04-REQ-1.E1: Propagate errors when operator is unreachable.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, instrument};

// ── Error types ────────────────────────────────────────────────────────────

/// Errors that can occur during operator REST calls.
#[derive(Debug, Error)]
pub enum OperatorError {
    /// HTTP request failed (network error, timeout, DNS, etc.).
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Operator returned a non-success HTTP status.
    #[error("Operator returned HTTP {status}: {body}")]
    Status {
        /// HTTP status code.
        status: u16,
        /// Response body text.
        body: String,
    },
}

/// Result type alias for operator operations.
pub type Result<T> = std::result::Result<T, OperatorError>;

// ── Request / Response types ───────────────────────────────────────────────

/// Request body for `POST /parking/start`.
#[derive(Debug, Serialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub zone_id: String,
    pub timestamp: i64,
}

/// Rate information included in start responses.
#[derive(Debug, Clone, Deserialize)]
pub struct RateInfo {
    pub zone_id: String,
    pub rate_type: String,
    pub rate_amount: f64,
    pub currency: String,
}

/// Response from `POST /parking/start`.
#[derive(Debug, Deserialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
    pub rate: RateInfo,
}

/// Request body for `POST /parking/stop`.
#[derive(Debug, Serialize)]
pub struct StopRequest {
    pub session_id: String,
    pub timestamp: i64,
}

/// Response from `POST /parking/stop`.
#[derive(Debug, Deserialize)]
pub struct StopResponse {
    pub session_id: String,
    pub status: String,
    pub total_fee: f64,
    pub duration_seconds: i64,
    pub currency: String,
}

/// Response from `GET /parking/rate`.
#[derive(Debug, Deserialize)]
pub struct RateResponse {
    pub zone_id: String,
    pub rate_type: String,
    pub rate_amount: f64,
    pub currency: String,
}

/// Rate info embedded in session response.
#[derive(Debug, Deserialize)]
pub struct SessionRateInfo {
    pub rate_type: String,
    pub rate_amount: f64,
    pub currency: String,
}

/// Response from `GET /parking/sessions/{id}`.
#[derive(Debug, Deserialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub vehicle_id: String,
    pub zone_id: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub rate: SessionRateInfo,
    pub total_fee: f64,
    pub duration_seconds: i64,
    pub status: String,
}

// ── Client ─────────────────────────────────────────────────────────────────

/// REST client for the PARKING_OPERATOR service.
///
/// Uses `reqwest` to make HTTP calls to the operator's REST API.
#[derive(Debug, Clone)]
pub struct OperatorClient {
    base_url: String,
    http: reqwest::Client,
}

impl OperatorClient {
    /// Create a new operator client with the given base URL.
    ///
    /// The base URL should not end with a trailing slash.
    /// Example: `http://localhost:8082`
    pub fn new(base_url: &str) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            base_url,
            http: reqwest::Client::new(),
        }
    }

    /// Start a parking session with the operator.
    ///
    /// Calls `POST /parking/start` with vehicle_id, zone_id, and timestamp.
    #[instrument(skip(self), fields(url = %self.base_url))]
    pub async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
        timestamp: i64,
    ) -> Result<StartResponse> {
        let url = format!("{}/parking/start", self.base_url);
        let body = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp,
        };

        debug!(?body, "POST /parking/start");

        let resp = self.http.post(&url).json(&body).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OperatorError::Status {
                status: status.as_u16(),
                body,
            });
        }

        Ok(resp.json().await?)
    }

    /// Stop a parking session with the operator.
    ///
    /// Calls `POST /parking/stop` with session_id and timestamp.
    #[instrument(skip(self), fields(url = %self.base_url))]
    pub async fn stop_session(
        &self,
        session_id: &str,
        timestamp: i64,
    ) -> Result<StopResponse> {
        let url = format!("{}/parking/stop", self.base_url);
        let body = StopRequest {
            session_id: session_id.to_string(),
            timestamp,
        };

        debug!(?body, "POST /parking/stop");

        let resp = self.http.post(&url).json(&body).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OperatorError::Status {
                status: status.as_u16(),
                body,
            });
        }

        Ok(resp.json().await?)
    }

    /// Query the parking rate for a zone.
    ///
    /// Calls `GET /parking/rate`.
    #[instrument(skip(self), fields(url = %self.base_url))]
    pub async fn get_rate(&self, _zone_id: &str) -> Result<RateResponse> {
        let url = format!("{}/parking/rate", self.base_url);

        debug!("GET /parking/rate");

        let resp = self.http.get(&url).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OperatorError::Status {
                status: status.as_u16(),
                body,
            });
        }

        Ok(resp.json().await?)
    }

    /// Get details for a specific parking session.
    ///
    /// Calls `GET /parking/sessions/{session_id}`.
    #[instrument(skip(self), fields(url = %self.base_url))]
    pub async fn get_session(&self, session_id: &str) -> Result<SessionResponse> {
        let url = format!("{}/parking/sessions/{}", self.base_url, session_id);

        debug!("GET /parking/sessions/{}", session_id);

        let resp = self.http.get(&url).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OperatorError::Status {
                status: status.as_u16(),
                body,
            });
        }

        Ok(resp.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Unit tests using wiremock ──────────────────────────────────────────
    //
    // These tests start a mock HTTP server and verify that the OperatorClient
    // sends the correct requests and parses responses properly.

    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn start_session_sends_correct_request() {
        let mock_server = MockServer::start().await;

        let expected_body = serde_json::json!({
            "vehicle_id": "DEMO0000000000001",
            "zone_id": "zone-1",
            "timestamp": 1708300800_i64
        });

        let response_body = serde_json::json!({
            "session_id": "sess-001",
            "status": "active",
            "rate": {
                "zone_id": "zone-1",
                "rate_type": "per_minute",
                "rate_amount": 0.05,
                "currency": "EUR"
            }
        });

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .and(body_json(&expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .start_session("DEMO0000000000001", "zone-1", 1_708_300_800)
            .await
            .unwrap();

        assert_eq!(resp.session_id, "sess-001");
        assert_eq!(resp.status, "active");
        assert_eq!(resp.rate.zone_id, "zone-1");
        assert_eq!(resp.rate.rate_type, "per_minute");
        assert!((resp.rate.rate_amount - 0.05).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");
    }

    #[tokio::test]
    async fn stop_session_sends_correct_request() {
        let mock_server = MockServer::start().await;

        let expected_body = serde_json::json!({
            "session_id": "sess-001",
            "timestamp": 1708301100_i64
        });

        let response_body = serde_json::json!({
            "session_id": "sess-001",
            "status": "completed",
            "total_fee": 0.25,
            "duration_seconds": 300,
            "currency": "EUR"
        });

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .and(body_json(&expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .stop_session("sess-001", 1_708_301_100)
            .await
            .unwrap();

        assert_eq!(resp.session_id, "sess-001");
        assert_eq!(resp.status, "completed");
        assert!((resp.total_fee - 0.25).abs() < f64::EPSILON);
        assert_eq!(resp.duration_seconds, 300);
        assert_eq!(resp.currency, "EUR");
    }

    #[tokio::test]
    async fn get_rate_returns_rate_info() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "zone_id": "zone-1",
            "rate_type": "per_minute",
            "rate_amount": 0.05,
            "currency": "EUR"
        });

        Mock::given(method("GET"))
            .and(path("/parking/rate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.get_rate("zone-1").await.unwrap();

        assert_eq!(resp.zone_id, "zone-1");
        assert_eq!(resp.rate_type, "per_minute");
        assert!((resp.rate_amount - 0.05).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "EUR");
    }

    #[tokio::test]
    async fn get_session_returns_session_details() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "session_id": "sess-001",
            "vehicle_id": "DEMO0000000000001",
            "zone_id": "zone-1",
            "start_time": 1708300800_i64,
            "end_time": 1708301100_i64,
            "rate": {
                "rate_type": "per_minute",
                "rate_amount": 0.05,
                "currency": "EUR"
            },
            "total_fee": 0.25,
            "duration_seconds": 300,
            "status": "completed"
        });

        Mock::given(method("GET"))
            .and(path("/parking/sessions/sess-001"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.get_session("sess-001").await.unwrap();

        assert_eq!(resp.session_id, "sess-001");
        assert_eq!(resp.vehicle_id, "DEMO0000000000001");
        assert_eq!(resp.zone_id, "zone-1");
        assert_eq!(resp.start_time, 1_708_300_800);
        assert_eq!(resp.end_time, Some(1_708_301_100));
        assert_eq!(resp.status, "completed");
        assert!((resp.total_fee - 0.25).abs() < f64::EPSILON);
        assert_eq!(resp.duration_seconds, 300);
    }

    #[tokio::test]
    async fn stop_session_unknown_id_returns_error() {
        let mock_server = MockServer::start().await;

        let error_body = serde_json::json!({"error": "session not found"});

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(404).set_body_json(&error_body))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let err = client
            .stop_session("unknown-session", 1_708_301_100)
            .await
            .unwrap_err();

        match err {
            OperatorError::Status { status, body } => {
                assert_eq!(status, 404);
                assert!(body.contains("session not found"));
            }
            other => panic!("expected Status error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn operator_unreachable_returns_http_error() {
        // Connect to a port that nothing is listening on
        let client = OperatorClient::new("http://127.0.0.1:1");
        let err = client
            .start_session("VIN1", "zone-1", 1_708_300_800)
            .await
            .unwrap_err();

        match err {
            OperatorError::Http(_) => {} // expected
            other => panic!("expected Http error, got: {:?}", other),
        }
    }

    #[test]
    fn client_new_trims_trailing_slash() {
        let client = OperatorClient::new("http://localhost:8082/");
        assert_eq!(client.base_url, "http://localhost:8082");
    }

    #[test]
    fn client_new_no_trailing_slash() {
        let client = OperatorClient::new("http://localhost:8082");
        assert_eq!(client.base_url, "http://localhost:8082");
    }

    #[test]
    fn operator_error_display() {
        let err = OperatorError::Status {
            status: 404,
            body: "not found".into(),
        };
        assert_eq!(err.to_string(), "Operator returned HTTP 404: not found");
    }
}
