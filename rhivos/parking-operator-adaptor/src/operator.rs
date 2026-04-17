//! PARKING_OPERATOR REST client.
//!
//! Sends `POST /parking/start` and `POST /parking/stop` to the PARKING_OPERATOR
//! REST API with exponential-backoff retry (08-REQ-2.1 through 08-REQ-2.E2).

#![allow(dead_code)]

use crate::session::Rate;
use serde::{Deserialize, Serialize};

// ── Public data types ─────────────────────────────────────────────────────────

/// Error returned when operator REST calls fail.
#[derive(Debug)]
pub enum OperatorError {
    /// All retry attempts exhausted (08-REQ-2.E1).
    RetriesExhausted(String),
    /// Response body could not be parsed.
    Parse(String),
    /// Other HTTP/network error.
    Other(String),
}

impl std::fmt::Display for OperatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorError::RetriesExhausted(msg) => write!(f, "retries exhausted: {msg}"),
            OperatorError::Parse(msg) => write!(f, "parse error: {msg}"),
            OperatorError::Other(msg) => write!(f, "operator error: {msg}"),
        }
    }
}

/// Parsed response from `POST /parking/start` (08-REQ-2.3).
#[derive(Debug, Clone)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
    pub rate: Rate,
}

/// Parsed response from `POST /parking/stop` (08-REQ-2.4).
#[derive(Debug, Clone)]
pub struct StopResponse {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: u64,
    pub total_amount: f64,
    pub currency: String,
}

// ── Internal serde types ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct StartRequest<'a> {
    vehicle_id: &'a str,
    zone_id: &'a str,
    timestamp: i64,
}

#[derive(Serialize)]
struct StopRequest<'a> {
    session_id: &'a str,
    timestamp: i64,
}

#[derive(Deserialize)]
struct StartResponseJson {
    session_id: String,
    status: String,
    rate: RateJson,
}

#[derive(Deserialize)]
struct RateJson {
    #[serde(rename = "type")]
    rate_type: String,
    amount: f64,
    currency: String,
}

#[derive(Deserialize)]
struct StopResponseJson {
    session_id: String,
    status: String,
    duration_seconds: u64,
    total_amount: f64,
    currency: String,
}

// ── OperatorClient ────────────────────────────────────────────────────────────

/// HTTP client for the PARKING_OPERATOR REST API (08-REQ-2.1 through 08-REQ-2.4).
pub struct OperatorClient {
    client: reqwest::Client,
    base_url: String,
}

impl OperatorClient {
    /// Create a new client for the operator at `base_url`.
    pub fn new(base_url: &str) -> Self {
        OperatorClient {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// POST /parking/start with retry (08-REQ-2.1, 08-REQ-2.E1, 08-REQ-2.E2).
    ///
    /// Sends `{vehicle_id, zone_id, timestamp}` and parses the response.
    /// Retries up to 3 times with delays of 1s, 2s, 4s on network or non-200 errors.
    pub async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        todo!("implement start_session with exponential-backoff retry")
    }

    /// POST /parking/stop with retry (08-REQ-2.2, 08-REQ-2.E1, 08-REQ-2.E2).
    ///
    /// Sends `{session_id, timestamp}` and parses the response.
    pub async fn stop_session(&self, _session_id: &str) -> Result<StopResponse, OperatorError> {
        todo!("implement stop_session with exponential-backoff retry")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn start_response_json() -> serde_json::Value {
        serde_json::json!({
            "session_id": "s1",
            "status": "active",
            "rate": {
                "type": "per_hour",
                "amount": 2.5,
                "currency": "EUR"
            }
        })
    }

    fn stop_response_json() -> serde_json::Value {
        serde_json::json!({
            "session_id": "sess-1",
            "status": "completed",
            "duration_seconds": 3600,
            "total_amount": 2.50,
            "currency": "EUR"
        })
    }

    /// TS-08-8: Correct POST /parking/start request and parsed response.
    ///
    /// Requires: 08-REQ-2.1, 08-REQ-2.3
    #[tokio::test]
    async fn test_start_session_request() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(start_response_json()))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .start_session("DEMO-VIN-001", "zone-a")
            .await
            .expect("start_session must succeed");

        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert_eq!(resp.rate.amount, 2.5);
        assert_eq!(resp.rate.currency, "EUR");
    }

    /// TS-08-9: Correct POST /parking/stop request and parsed response.
    ///
    /// Requires: 08-REQ-2.2, 08-REQ-2.4
    #[tokio::test]
    async fn test_stop_session_request() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(stop_response_json()))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .stop_session("sess-1")
            .await
            .expect("stop_session must succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert_eq!(resp.total_amount, 2.50);
        assert_eq!(resp.currency, "EUR");
    }

    /// TS-08-10: Start response is correctly parsed into session state fields.
    ///
    /// Requires: 08-REQ-2.3
    #[tokio::test]
    async fn test_start_response_parsing() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "s1",
                "status": "active",
                "rate": {
                    "type": "per_hour",
                    "amount": 2.5,
                    "currency": "EUR"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .start_session("DEMO-VIN-001", "zone-a")
            .await
            .expect("parse must succeed");

        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert_eq!(resp.rate.amount, 2.5);
        assert_eq!(resp.status, "active");
    }

    /// TS-08-E3: Retries when operator fails first 2 calls, succeeds on 3rd.
    ///
    /// Requires: 08-REQ-2.E1
    #[tokio::test]
    async fn test_retry_on_failure() {
        let mock_server = MockServer::start().await;
        // Fail first 2, succeed on 3rd.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(start_response_json()))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_ok(), "must succeed after retries");
        // 3 requests total (2 failures + 1 success).
        let received = mock_server.received_requests().await.unwrap();
        assert_eq!(received.len(), 3, "must send exactly 3 requests");
    }

    /// TS-08-E4: Returns error after all 4 attempts (1 initial + 3 retries) fail.
    ///
    /// Requires: 08-REQ-2.E1
    #[tokio::test]
    async fn test_retry_exhausted() {
        let mock_server = MockServer::start().await;
        // Always fail.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_err(), "must return error after retries exhausted");
        // 4 total requests: 1 initial + 3 retries.
        let received = mock_server.received_requests().await.unwrap();
        assert_eq!(received.len(), 4, "must attempt exactly 4 times");
    }

    /// TS-08-E5: Non-200 HTTP responses trigger retry logic.
    ///
    /// Requires: 08-REQ-2.E2
    #[tokio::test]
    async fn test_retry_on_non_200() {
        let mock_server = MockServer::start().await;
        // Return 500 twice, then 200.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(start_response_json()))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_ok(), "must succeed after non-200 retries");
        let received = mock_server.received_requests().await.unwrap();
        assert_eq!(received.len(), 3, "must send exactly 3 requests");
    }
}
