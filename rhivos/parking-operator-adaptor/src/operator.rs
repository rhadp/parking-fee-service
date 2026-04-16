//! PARKING_OPERATOR REST client with exponential-backoff retry.

use serde::{Deserialize, Serialize};

use crate::session::Rate;

// ─── Error types ─────────────────────────────────────────────────────────────

/// Errors returned by operator REST calls.
#[derive(Debug)]
pub enum OperatorError {
    RequestFailed(String),
    ParseError(String),
    /// All retry attempts exhausted.
    RetriesExhausted,
}

// ─── REST request/response models ────────────────────────────────────────────

/// Body sent to POST /parking/start.
#[derive(Debug, Serialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub zone_id: String,
    pub timestamp: i64,
}

/// Rate object inside the start response.
/// The JSON field is named "type" but we store it as `rate_type`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RateResponse {
    #[serde(rename = "type")]
    pub rate_type: String,
    pub amount: f64,
    pub currency: String,
}

impl From<RateResponse> for Rate {
    fn from(r: RateResponse) -> Self {
        Rate {
            rate_type: r.rate_type,
            amount: r.amount,
            currency: r.currency,
        }
    }
}

/// Parsed response from POST /parking/start.
#[derive(Debug, Deserialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
    pub rate: RateResponse,
}

/// Body sent to POST /parking/stop.
#[derive(Debug, Serialize)]
pub struct StopRequest {
    pub session_id: String,
    pub timestamp: i64,
}

/// Parsed response from POST /parking/stop.
#[derive(Debug, Deserialize)]
pub struct StopResponse {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: u64,
    pub total_amount: f64,
    pub currency: String,
}

// ─── OperatorApi trait ───────────────────────────────────────────────────────

/// Abstraction over the PARKING_OPERATOR REST API.
///
/// Implemented by `OperatorClient` (production) and mocks in tests.
#[allow(async_fn_in_trait)]
pub trait OperatorApi {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError>;

    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError>;
}

// ─── Production client ───────────────────────────────────────────────────────

/// HTTP client for the PARKING_OPERATOR REST API.
#[allow(dead_code)]
pub struct OperatorClient {
    base_url: String,
    client: reqwest::Client,
}

impl OperatorClient {
    /// Create a new client with the given base URL (e.g. "http://localhost:8080").
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            client: reqwest::Client::new(),
        }
    }
}

impl OperatorApi for OperatorClient {
    async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        todo!("OperatorClient::start_session not yet implemented")
    }

    async fn stop_session(&self, _session_id: &str) -> Result<StopResponse, OperatorError> {
        todo!("OperatorClient::stop_session not yet implemented")
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn start_body() -> serde_json::Value {
        serde_json::json!({
            "session_id": "s1",
            "status": "active",
            "rate": {"type": "per_hour", "amount": 2.5, "currency": "EUR"}
        })
    }

    fn stop_body() -> serde_json::Value {
        serde_json::json!({
            "session_id": "sess-1",
            "status": "completed",
            "duration_seconds": 3600,
            "total_amount": 2.50,
            "currency": "EUR"
        })
    }

    // TS-08-8: Operator Start Session REST Call
    #[tokio::test]
    async fn test_start_session_request() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(start_body()))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .start_session("DEMO-VIN-001", "zone-a")
            .await
            .expect("start_session should succeed");

        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert_eq!(resp.rate.amount, 2.5);
        assert_eq!(resp.rate.currency, "EUR");

        // Verify the request was actually sent.
        let received = mock_server.received_requests().await.unwrap();
        assert_eq!(received.len(), 1);
        let body: serde_json::Value =
            serde_json::from_slice(&received[0].body).expect("body should be JSON");
        assert!(body.get("vehicle_id").is_some(), "body must contain vehicle_id");
        assert!(body.get("zone_id").is_some(), "body must contain zone_id");
        assert!(body.get("timestamp").is_some(), "body must contain timestamp");
    }

    // TS-08-9: Operator Stop Session REST Call
    #[tokio::test]
    async fn test_stop_session_request() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(stop_body()))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .stop_session("sess-1")
            .await
            .expect("stop_session should succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert_eq!(resp.total_amount, 2.50);
        assert_eq!(resp.currency, "EUR");

        let received = mock_server.received_requests().await.unwrap();
        assert_eq!(received.len(), 1);
        let body: serde_json::Value =
            serde_json::from_slice(&received[0].body).expect("body should be JSON");
        assert!(body.get("session_id").is_some(), "body must contain session_id");
        assert!(body.get("timestamp").is_some(), "body must contain timestamp");
    }

    // TS-08-10: Operator Start Response Parsing
    #[tokio::test]
    async fn test_start_response_parsing() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "s1",
                "status": "active",
                "rate": {"type": "per_hour", "amount": 2.5, "currency": "EUR"}
            })))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .start_session("DEMO-VIN-001", "zone-a")
            .await
            .expect("start_session should succeed");

        // Verify rate parsing (the "type" field renamed to rate_type).
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert_eq!(resp.rate.amount, 2.5);
    }

    // TS-08-E3: Operator REST Retry on Failure (fail 2×, succeed on 3rd)
    #[tokio::test]
    async fn test_retry_on_failure() {
        let mock_server = MockServer::start().await;

        // First two requests return connection error via 503.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        // Third request succeeds.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(start_body()))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .start_session("VIN", "zone")
            .await
            .expect("should succeed after retries");

        assert_eq!(resp.session_id, "s1");

        let received = mock_server.received_requests().await.unwrap();
        assert_eq!(received.len(), 3, "exactly 3 requests: 1 initial + 2 retries");
    }

    // TS-08-E4: Operator REST All Retries Exhausted
    #[tokio::test]
    async fn test_retry_exhausted() {
        let mock_server = MockServer::start().await;

        // All requests fail.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let result = client.start_session("VIN", "zone").await;

        assert!(result.is_err(), "should return Err after all retries fail");
        assert!(
            matches!(result.unwrap_err(), OperatorError::RetriesExhausted),
            "error should be RetriesExhausted"
        );

        let received = mock_server.received_requests().await.unwrap();
        assert_eq!(
            received.len(),
            4,
            "exactly 4 requests: 1 initial + 3 retries"
        );
    }

    // TS-08-E5: Operator Non-200 Status Triggers Retry
    #[tokio::test]
    async fn test_retry_on_non_200() {
        let mock_server = MockServer::start().await;

        // First two requests return 500.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        // Third request returns 200.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(start_body()))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .start_session("VIN", "zone")
            .await
            .expect("should succeed after retries");

        assert_eq!(resp.session_id, "s1");

        let received = mock_server.received_requests().await.unwrap();
        assert_eq!(received.len(), 3, "3 requests: 500, 500, 200");
    }
}
