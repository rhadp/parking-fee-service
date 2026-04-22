use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use tracing::{info, warn};

/// Maximum number of retries after the initial attempt.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff in milliseconds.
/// Retry delays: base * 2^0, base * 2^1, base * 2^2 → 1s, 2s, 4s.
#[cfg(not(test))]
const RETRY_BASE_MS: u64 = 1000;

/// Shortened base delay for tests to avoid multi-second waits.
#[cfg(test)]
const RETRY_BASE_MS: u64 = 10;

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
struct StartRequest {
    vehicle_id: String,
    zone_id: String,
    timestamp: i64,
}

/// Request body for POST /parking/stop.
#[derive(Debug, Serialize)]
struct StopRequest {
    session_id: String,
    timestamp: i64,
}

/// REST client for the PARKING_OPERATOR backend.
///
/// Sends start/stop requests with retry logic (up to 3 retries,
/// exponential backoff: 1s, 2s, 4s).
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
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        let url = format!("{}/parking/start", self.base_url);
        let body = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp: now_unix_timestamp(),
        };

        let response_bytes = self.post_with_retry(&url, &body).await?;
        serde_json::from_slice::<StartResponse>(&response_bytes)
            .map_err(|e| OperatorError::ParseError(e.to_string()))
    }

    /// Stop a parking session with the PARKING_OPERATOR.
    ///
    /// Sends POST /parking/stop with {session_id, timestamp}.
    /// Retries up to 3 times with exponential backoff on failure.
    pub async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        let url = format!("{}/parking/stop", self.base_url);
        let body = StopRequest {
            session_id: session_id.to_string(),
            timestamp: now_unix_timestamp(),
        };

        let response_bytes = self.post_with_retry(&url, &body).await?;
        serde_json::from_slice::<StopResponse>(&response_bytes)
            .map_err(|e| OperatorError::ParseError(e.to_string()))
    }

    /// Send a POST request with retry logic.
    ///
    /// Retries up to `MAX_RETRIES` times with exponential backoff
    /// (RETRY_BASE_MS * 2^attempt) on connection error, timeout, or non-200
    /// HTTP status.
    async fn post_with_retry<T: Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<Vec<u8>, OperatorError> {
        let mut last_error = OperatorError::RequestFailed("no attempts made".to_string());

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay_ms = RETRY_BASE_MS * 2u64.pow(attempt - 1);
                info!(attempt, delay_ms, url, "retrying operator request");
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }

            match self.send_post(url, body).await {
                Ok(bytes) => return Ok(bytes),
                Err(e) => {
                    warn!(
                        attempt = attempt + 1,
                        max_attempts = MAX_RETRIES + 1,
                        error = %e,
                        url,
                        "operator request failed"
                    );
                    last_error = e;
                }
            }
        }

        Err(last_error)
    }

    /// Send a single POST request and check the response status.
    ///
    /// Returns the response body bytes on 200, or an OperatorError otherwise.
    async fn send_post<T: Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<Vec<u8>, OperatorError> {
        let response = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| OperatorError::RequestFailed(e.to_string()))?;

        let status = response.status().as_u16();
        if status != 200 {
            let body_text = response.text().await.unwrap_or_default();
            return Err(OperatorError::HttpError(status, body_text));
        }

        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| OperatorError::RequestFailed(e.to_string()))?;

        Ok(body_bytes.to_vec())
    }
}

/// Get the current Unix timestamp in seconds.
fn now_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs() as i64
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
