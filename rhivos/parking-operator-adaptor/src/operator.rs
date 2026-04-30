use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Maximum number of retries after the initial attempt.
const MAX_RETRIES: usize = 3;

/// Retry delays in milliseconds for exponential backoff.
/// In test builds, use small values to keep tests fast.
#[cfg(test)]
const RETRY_DELAYS_MS: [u64; MAX_RETRIES] = [10, 20, 40];
#[cfg(not(test))]
const RETRY_DELAYS_MS: [u64; MAX_RETRIES] = [1000, 2000, 4000];

/// Operator REST start request body.
#[derive(Debug, Serialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub zone_id: String,
    pub timestamp: i64,
}

/// Rate information from the operator.
#[derive(Debug, Deserialize, Clone)]
pub struct RateResponse {
    #[serde(rename = "type")]
    pub rate_type: String,
    pub amount: f64,
    pub currency: String,
}

/// Operator REST start response body.
#[derive(Debug, Deserialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
    pub rate: RateResponse,
}

/// Operator REST stop request body.
#[derive(Debug, Serialize)]
pub struct StopRequest {
    pub session_id: String,
    pub timestamp: i64,
}

/// Operator REST stop response body.
#[derive(Debug, Deserialize)]
pub struct StopResponse {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: u64,
    pub total_amount: f64,
    pub currency: String,
}

/// Error type for operator REST client operations.
#[derive(Debug)]
pub enum OperatorError {
    /// HTTP request failed after all retries.
    RequestFailed(String),
    /// Response parsing failed.
    ParseError(String),
}

impl std::fmt::Display for OperatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorError::RequestFailed(msg) => write!(f, "operator request failed: {msg}"),
            OperatorError::ParseError(msg) => write!(f, "operator response parse error: {msg}"),
        }
    }
}

impl std::error::Error for OperatorError {}

/// Returns the current Unix timestamp in seconds.
fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs() as i64
}

/// REST client for the PARKING_OPERATOR backend.
///
/// Sends start/stop requests and implements retry logic with
/// exponential backoff (1s, 2s, 4s) on failure.
pub struct OperatorClient {
    client: reqwest::Client,
    base_url: String,
}

impl OperatorClient {
    /// Create a new operator client with the given base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Start a parking session with the operator.
    ///
    /// Sends `POST /parking/start` with `{vehicle_id, zone_id, timestamp}`.
    /// Retries up to 3 times with exponential backoff (1s, 2s, 4s) on
    /// failure or non-200 status.
    pub async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        let url = format!("{}/parking/start", self.base_url);
        let body = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp: unix_timestamp(),
        };
        self.post_with_retry(&url, &body).await
    }

    /// Stop a parking session with the operator.
    ///
    /// Sends `POST /parking/stop` with `{session_id, timestamp}`.
    /// Retries up to 3 times with exponential backoff (1s, 2s, 4s) on
    /// failure or non-200 status.
    pub async fn stop_session(
        &self,
        session_id: &str,
    ) -> Result<StopResponse, OperatorError> {
        let url = format!("{}/parking/stop", self.base_url);
        let body = StopRequest {
            session_id: session_id.to_string(),
            timestamp: unix_timestamp(),
        };
        self.post_with_retry(&url, &body).await
    }

    /// Send a POST request with JSON body and retry on failure.
    ///
    /// Retries up to `MAX_RETRIES` times with exponential backoff
    /// (`RETRY_DELAYS_MS`) on connection errors or non-success HTTP
    /// status codes. Parse errors on a successful response are returned
    /// immediately without retrying.
    async fn post_with_retry<Req, Resp>(
        &self,
        url: &str,
        body: &Req,
    ) -> Result<Resp, OperatorError>
    where
        Req: Serialize,
        Resp: for<'de> Deserialize<'de>,
    {
        let mut last_error = String::new();

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = Duration::from_millis(RETRY_DELAYS_MS[attempt - 1]);
                tracing::warn!(
                    attempt = attempt + 1,
                    max_attempts = MAX_RETRIES + 1,
                    error = %last_error,
                    "retrying operator request after failure"
                );
                tokio::time::sleep(delay).await;
            }

            match self.client.post(url).json(body).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return response
                            .json::<Resp>()
                            .await
                            .map_err(|e| OperatorError::ParseError(e.to_string()));
                    }
                    last_error = format!("HTTP {}", response.status());
                }
                Err(e) => {
                    last_error = e.to_string();
                }
            }
        }

        tracing::error!(
            error = %last_error,
            "operator request failed after all retries"
        );
        Err(OperatorError::RequestFailed(last_error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, body_string_contains};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // TS-08-8: Operator Start Session REST Call
    // Verify correct POST /parking/start with body and response parsing.
    #[tokio::test]
    async fn test_start_session_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .and(body_string_contains("vehicle_id"))
            .and(body_string_contains("zone_id"))
            .and(body_string_contains("timestamp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "s1",
                "status": "active",
                "rate": {
                    "type": "per_hour",
                    "amount": 2.50,
                    "currency": "EUR"
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .start_session("DEMO-VIN-001", "zone-a")
            .await
            .expect("start_session should succeed");

        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");
    }

    // TS-08-9: Operator Stop Session REST Call
    // Verify correct POST /parking/stop with body and response parsing.
    #[tokio::test]
    async fn test_stop_session_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .and(body_string_contains("session_id"))
            .and(body_string_contains("timestamp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-1",
                "status": "completed",
                "duration_seconds": 3600,
                "total_amount": 2.50,
                "currency": "EUR"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client
            .stop_session("sess-1")
            .await
            .expect("stop_session should succeed");

        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert!((resp.total_amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "EUR");
    }

    // TS-08-10: Operator Start Response Parsing
    // Verify the start response is parsed into StartResponse.
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
        let resp = client.start_session("VIN", "zone").await.unwrap();
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.status, "active");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.5).abs() < f64::EPSILON);
    }

    // TS-08-E3: Operator REST Retry on Failure
    // Verify the adaptor retries operator REST calls, succeeding on 3rd attempt.
    #[tokio::test]
    async fn test_retry_on_failure() {
        let mock_server = MockServer::start().await;

        // First 2 calls fail with 500, 3rd succeeds.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "s1",
                "status": "active",
                "rate": { "type": "per_hour", "amount": 2.5, "currency": "EUR" }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_ok(), "should succeed after retries");
    }

    // TS-08-E4: Operator REST All Retries Exhausted
    // Verify error returned after all retries fail.
    #[tokio::test]
    async fn test_retry_exhausted() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .expect(4) // 1 initial + 3 retries
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_err(), "should fail after all retries exhausted");
    }

    // TS-08-E5: Operator Non-200 Status Triggers Retry
    // Verify non-200 HTTP responses trigger retry logic.
    #[tokio::test]
    async fn test_retry_on_non_200() {
        let mock_server = MockServer::start().await;

        // Return 500 twice, then 200 with valid body.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "s1",
                "status": "active",
                "rate": { "type": "per_hour", "amount": 2.5, "currency": "EUR" }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_ok(), "should succeed after non-200 retries");
    }
}
