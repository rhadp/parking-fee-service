use serde::{Deserialize, Serialize};
use std::fmt;

/// Error type for PARKING_OPERATOR REST client operations.
#[derive(Debug)]
pub enum OperatorError {
    /// HTTP request failed (connection error, timeout, etc.).
    RequestFailed(String),
    /// Non-200 HTTP status code.
    HttpError(u16, String),
    /// Response parsing failed.
    ParseError(String),
}

impl fmt::Display for OperatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperatorError::RequestFailed(msg) => write!(f, "request failed: {msg}"),
            OperatorError::HttpError(code, msg) => write!(f, "HTTP {code}: {msg}"),
            OperatorError::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for OperatorError {}

/// Rate information from the PARKING_OPERATOR start response.
#[derive(Debug, Clone, Deserialize)]
pub struct RateResponse {
    #[serde(rename = "type")]
    pub rate_type: String,
    pub amount: f64,
    pub currency: String,
}

/// Response from POST /parking/start.
#[derive(Debug, Clone, Deserialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
    pub rate: RateResponse,
}

/// Response from POST /parking/stop.
#[derive(Debug, Clone, Deserialize)]
pub struct StopResponse {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: u64,
    pub total_amount: f64,
    pub currency: String,
}

/// Request body for POST /parking/start.
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct StartRequest {
    vehicle_id: String,
    zone_id: String,
    timestamp: i64,
}

/// Request body for POST /parking/stop.
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct StopRequest {
    session_id: String,
    timestamp: i64,
}

/// REST client for the PARKING_OPERATOR backend.
///
/// Sends start/stop requests with retry logic (up to 3 retries,
/// exponential backoff: 1s, 2s, 4s).
#[allow(dead_code)]
pub struct OperatorClient {
    client: reqwest::Client,
    base_url: String,
}

impl OperatorClient {
    /// Create a new OperatorClient for the given base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Start a parking session with the PARKING_OPERATOR.
    ///
    /// Sends POST /parking/start with {vehicle_id, zone_id, timestamp}.
    /// Retries up to 3 times with exponential backoff on failure.
    pub async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        todo!("implement start_session")
    }

    /// Stop a parking session with the PARKING_OPERATOR.
    ///
    /// Sends POST /parking/stop with {session_id, timestamp}.
    /// Retries up to 3 times with exponential backoff on failure.
    pub async fn stop_session(&self, _session_id: &str) -> Result<StopResponse, OperatorError> {
        todo!("implement stop_session")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-08-8: Verify POST /parking/start sends correct request and parses response.
    #[tokio::test]
    async fn test_start_session_request() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/parking/start"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
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
            .expect("start_session should succeed");

        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.status, "active");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.5).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");

        // Verify the mock received the request.
        let requests = mock_server.received_requests().await.unwrap();
        assert!(!requests.is_empty(), "mock should have received at least one request");

        // Verify request body contains vehicle_id and zone_id.
        let body: serde_json::Value =
            serde_json::from_slice(&requests[0].body).expect("request body should be valid JSON");
        assert_eq!(body["vehicle_id"], "DEMO-VIN-001");
        assert_eq!(body["zone_id"], "zone-a");
        assert!(body["timestamp"].is_number(), "timestamp should be a number");
    }

    // TS-08-9: Verify POST /parking/stop sends correct request and parses response.
    #[tokio::test]
    async fn test_stop_session_request() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/parking/stop"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-1",
                "status": "completed",
                "duration_seconds": 3600,
                "total_amount": 2.50,
                "currency": "EUR"
            })))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .stop_session("sess-1")
            .await
            .expect("stop_session should succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.status, "completed");
        assert_eq!(resp.duration_seconds, 3600);
        assert!((resp.total_amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "EUR");

        // Verify request body.
        let requests = mock_server.received_requests().await.unwrap();
        assert!(!requests.is_empty());
        let body: serde_json::Value =
            serde_json::from_slice(&requests[0].body).expect("request body should be valid JSON");
        assert_eq!(body["session_id"], "sess-1");
        assert!(body["timestamp"].is_number());
    }

    // TS-08-10: Verify start response is parsed correctly.
    #[tokio::test]
    async fn test_start_response_parsing() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/parking/start"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
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
            .start_session("VIN", "zone")
            .await
            .expect("should parse response");

        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.5).abs() < f64::EPSILON);
    }

    // TS-08-E3: Verify retry on failure (fail first 2, succeed on 3rd).
    #[tokio::test]
    async fn test_retry_on_failure() {
        let mock_server = wiremock::MockServer::start().await;

        // Respond with 500 for first 2 calls, then 200.
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/parking/start"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/parking/start"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "s1",
                "status": "active",
                "rate": {
                    "type": "per_hour",
                    "amount": 2.5,
                    "currency": "EUR"
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_ok(), "should succeed after retries");

        let requests = mock_server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 3, "should have made 3 total requests");
    }

    // TS-08-E4: Verify error after all retries exhausted.
    #[tokio::test]
    async fn test_retry_exhausted() {
        let mock_server = wiremock::MockServer::start().await;

        // Always fail.
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/parking/start"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .expect(4) // 1 initial + 3 retries = 4 total
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_err(), "should fail after all retries exhausted");

        let requests = mock_server.received_requests().await.unwrap();
        assert_eq!(
            requests.len(),
            4,
            "should have made 4 total requests (1 initial + 3 retries)"
        );
    }

    // TS-08-E5: Verify non-200 HTTP status triggers retry.
    #[tokio::test]
    async fn test_retry_on_non_200() {
        let mock_server = wiremock::MockServer::start().await;

        // Return 500 twice, then 200.
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/parking/start"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/parking/start"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "s1",
                "status": "active",
                "rate": {
                    "type": "per_hour",
                    "amount": 2.5,
                    "currency": "EUR"
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_ok(), "should succeed after non-200 retries");

        let requests = mock_server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 3, "should have made 3 total requests");
    }
}
