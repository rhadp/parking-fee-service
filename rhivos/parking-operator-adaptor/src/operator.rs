use std::fmt;

use serde::{Deserialize, Serialize};

/// Operator REST start response.
#[derive(Debug, Clone, Deserialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
    pub rate: RateResponse,
}

/// Rate information from the operator response.
///
/// The JSON key is `"type"` but Rust reserves that keyword, so we rename it.
#[derive(Debug, Clone, Deserialize)]
pub struct RateResponse {
    #[serde(rename = "type")]
    pub rate_type: String,
    pub amount: f64,
    pub currency: String,
}

/// Operator REST stop response.
#[derive(Debug, Clone, Deserialize)]
pub struct StopResponse {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: u64,
    pub total_amount: f64,
    pub currency: String,
}

/// Operator REST start request body.
#[derive(Debug, Serialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub zone_id: String,
    pub timestamp: i64,
}

/// Operator REST stop request body.
#[derive(Debug, Serialize)]
pub struct StopRequest {
    pub session_id: String,
    pub timestamp: i64,
}

/// Error type for operator REST client operations.
#[derive(Debug)]
pub enum OperatorError {
    /// HTTP or network error.
    Http(String),
    /// Response parsing error.
    Parse(String),
}

impl fmt::Display for OperatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperatorError::Http(msg) => write!(f, "HTTP error: {msg}"),
            OperatorError::Parse(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for OperatorError {}

/// Trait for parking operator backend communication.
///
/// Abstracting the operator client behind a trait allows event_loop tests
/// to inject a mock implementation.
pub trait ParkingOperator {
    /// Start a parking session with the operator.
    fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> impl std::future::Future<Output = Result<StartResponse, OperatorError>>;

    /// Stop an active parking session with the operator.
    fn stop_session(
        &self,
        session_id: &str,
    ) -> impl std::future::Future<Output = Result<StopResponse, OperatorError>>;
}

/// HTTP client for the PARKING_OPERATOR REST API.
///
/// Uses reqwest with retry logic (up to 3 retries, exponential backoff
/// 1s, 2s, 4s) for transient failures and non-200 responses.
pub struct OperatorClient {
    client: reqwest::Client,
    base_url: String,
    retry_delays: Vec<std::time::Duration>,
}

/// Production retry delays for exponential backoff: 1s, 2s, 4s.
const DEFAULT_RETRY_DELAYS: [std::time::Duration; 3] = [
    std::time::Duration::from_secs(1),
    std::time::Duration::from_secs(2),
    std::time::Duration::from_secs(4),
];

impl OperatorClient {
    /// Create a new operator client with the given base URL.
    ///
    /// Uses the default retry delays (1s, 2s, 4s exponential backoff).
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            retry_delays: DEFAULT_RETRY_DELAYS.to_vec(),
        }
    }

    /// Create a new operator client with custom retry delays (for testing).
    #[cfg(test)]
    fn with_retry_delays(base_url: &str, retry_delays: Vec<std::time::Duration>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            retry_delays,
        }
    }

    /// Execute an HTTP POST with retry logic.
    ///
    /// Retries up to `self.retry_delays.len()` times with exponential backoff
    /// on connection errors, timeouts, or non-200 HTTP status codes.
    async fn post_with_retry<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, OperatorError> {
        let url = format!("{}{}", self.base_url, path);
        let mut last_error: Option<OperatorError> = None;

        // Initial attempt + up to N retries
        for attempt in 0..=self.retry_delays.len() {
            // Wait before retry (not before the first attempt)
            if attempt > 0 {
                tokio::time::sleep(self.retry_delays[attempt - 1]).await;
            }

            let result = self
                .client
                .post(&url)
                .json(body)
                .send()
                .await;

            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        return response
                            .json::<T>()
                            .await
                            .map_err(|e| OperatorError::Parse(e.to_string()));
                    }
                    // Non-200 status — treat as failure, retry
                    last_error = Some(OperatorError::Http(format!(
                        "non-200 status: {}",
                        response.status()
                    )));
                }
                Err(e) => {
                    last_error = Some(OperatorError::Http(e.to_string()));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| OperatorError::Http("unknown error".to_string())))
    }
}

impl ParkingOperator for OperatorClient {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let request = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp,
        };

        self.post_with_retry("/parking/start", &request).await
    }

    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let request = StopRequest {
            session_id: session_id.to_string(),
            timestamp,
        };

        self.post_with_retry("/parking/stop", &request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Zero-delay retries for fast unit tests (same retry count as production).
    fn test_retry_delays() -> Vec<Duration> {
        vec![Duration::ZERO; 3]
    }

    // TS-08-8: Operator Start Session REST Call
    // Validates: [08-REQ-2.1]
    #[tokio::test]
    async fn test_start_session_request() {
        // GIVEN a mock HTTP server
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "session_id": "s1",
            "status": "active",
            "rate": {
                "type": "per_hour",
                "amount": 2.5,
                "currency": "EUR"
            }
        });

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .and(body_partial_json(serde_json::json!({
                "vehicle_id": "DEMO-VIN-001",
                "zone_id": "zone-a"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        // WHEN start_session is called
        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .start_session("DEMO-VIN-001", "zone-a")
            .await
            .expect("start_session should succeed");

        // THEN the response is correctly parsed
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.5).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");
    }

    // TS-08-9: Operator Stop Session REST Call
    // Validates: [08-REQ-2.2], [08-REQ-2.4]
    #[tokio::test]
    async fn test_stop_session_request() {
        // GIVEN a mock HTTP server
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "session_id": "sess-1",
            "status": "completed",
            "duration_seconds": 3600,
            "total_amount": 2.50,
            "currency": "EUR"
        });

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .and(body_partial_json(serde_json::json!({
                "session_id": "sess-1"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        // WHEN stop_session is called
        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .stop_session("sess-1")
            .await
            .expect("stop_session should succeed");

        // THEN the response is correctly parsed
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert!((resp.total_amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "EUR");
    }

    // TS-08-10: Operator Start Response Parsing
    // Validates: [08-REQ-2.3]
    #[tokio::test]
    async fn test_start_response_parsing() {
        // GIVEN a mock HTTP server returning a valid start response
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "session_id": "s1",
            "status": "active",
            "rate": {
                "type": "per_hour",
                "amount": 2.5,
                "currency": "EUR"
            }
        });

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // WHEN start_session is called
        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await.unwrap();

        // THEN the response is parsed into the correct types
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.status, "active");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.5).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");
    }

    // TS-08-E3: Operator REST Retry on Failure
    // Validates: [08-REQ-2.E1]
    #[tokio::test]
    async fn test_retry_on_failure() {
        // GIVEN a mock HTTP server that fails first 2 calls, succeeds on 3rd
        let mock_server = MockServer::start().await;

        // First 2 requests return 500
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        // Subsequent requests return 200
        let response_body = serde_json::json!({
            "session_id": "s1",
            "status": "active",
            "rate": {
                "type": "per_hour",
                "amount": 2.5,
                "currency": "EUR"
            }
        });
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        // WHEN start_session is called (with zero-delay retries for fast test)
        let client = OperatorClient::with_retry_delays(&mock_server.uri(), test_retry_delays());
        let resp = client.start_session("VIN", "zone").await;

        // THEN the call succeeds after retries (3 total requests: 2 failures + 1 success)
        assert!(resp.is_ok());
        assert_eq!(resp.unwrap().session_id, "s1");
        // Expect counts are verified by wiremock on drop (2 failures + 1 success = 3 total)
    }

    // TS-08-E4: Operator REST All Retries Exhausted
    // Validates: [08-REQ-2.E1]
    #[tokio::test]
    async fn test_retry_exhausted() {
        // GIVEN a mock HTTP server that always fails
        let mock_server = MockServer::start().await;

        // 4 total attempts expected: 1 initial + 3 retries
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .expect(4)
            .mount(&mock_server)
            .await;

        // WHEN start_session is called (with zero-delay retries for fast test)
        let client = OperatorClient::with_retry_delays(&mock_server.uri(), test_retry_delays());
        let resp = client.start_session("VIN", "zone").await;

        // THEN error is returned after all retries
        assert!(resp.is_err());
        // Expect count (4) verified by wiremock on drop
    }

    // TS-08-E5: Operator Non-200 Status Triggers Retry
    // Validates: [08-REQ-2.E2]
    #[tokio::test]
    async fn test_retry_on_non_200() {
        // GIVEN a mock HTTP server returning 500 twice, then 200
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        let response_body = serde_json::json!({
            "session_id": "s1",
            "status": "active",
            "rate": {
                "type": "per_hour",
                "amount": 2.5,
                "currency": "EUR"
            }
        });
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        // WHEN start_session is called (with zero-delay retries for fast test)
        let client = OperatorClient::with_retry_delays(&mock_server.uri(), test_retry_delays());
        let resp = client.start_session("VIN", "zone").await;

        // THEN the call succeeds after retries (3 total requests)
        assert!(resp.is_ok());
        // Expect counts verified by wiremock on drop (2 + 1 = 3)
    }
}
