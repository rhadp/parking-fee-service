use serde::{Deserialize, Serialize};
use std::fmt;

/// Error type for PARKING_OPERATOR REST client operations.
#[derive(Debug)]
pub enum OperatorError {
    /// All retry attempts exhausted.
    RetriesExhausted(String),
    /// Response parsing failed.
    ParseError(String),
    /// HTTP request error.
    HttpError(String),
}

impl fmt::Display for OperatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperatorError::RetriesExhausted(msg) => write!(f, "retries exhausted: {msg}"),
            OperatorError::ParseError(msg) => write!(f, "parse error: {msg}"),
            OperatorError::HttpError(msg) => write!(f, "HTTP error: {msg}"),
        }
    }
}

impl std::error::Error for OperatorError {}

/// Request body for POST /parking/start.
#[derive(Debug, Serialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub zone_id: String,
    pub timestamp: i64,
}

/// Response from POST /parking/start.
#[derive(Debug, Clone, Deserialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
    pub rate: RateResponse,
}

/// Rate portion of the start response.
#[derive(Debug, Clone, Deserialize)]
pub struct RateResponse {
    #[serde(rename = "type")]
    pub rate_type: String,
    pub amount: f64,
    pub currency: String,
}

/// Request body for POST /parking/stop.
#[derive(Debug, Serialize)]
pub struct StopRequest {
    pub session_id: String,
    pub timestamp: i64,
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

/// Trait abstracting the PARKING_OPERATOR REST client for testability.
#[allow(async_fn_in_trait)]
pub trait ParkingOperator {
    /// Start a parking session with the operator.
    /// Retries up to 3 times with exponential backoff (1s, 2s, 4s).
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError>;

    /// Stop a parking session with the operator.
    /// Retries up to 3 times with exponential backoff (1s, 2s, 4s).
    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError>;
}

/// Maximum number of retries after the initial attempt.
const MAX_RETRIES: usize = 3;

/// Backoff delays for retries: 1s, 2s, 4s.
const BACKOFF_DELAYS_MS: [u64; 3] = [1000, 2000, 4000];

/// HTTP-backed PARKING_OPERATOR REST client.
pub struct OperatorClient {
    client: reqwest::Client,
    base_url: String,
}

impl OperatorClient {
    /// Create a new OperatorClient targeting the given base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Execute an HTTP POST with JSON body and retry logic.
    ///
    /// Retries up to `MAX_RETRIES` times with exponential backoff on
    /// connection errors, timeouts, or non-200 HTTP status codes.
    async fn post_with_retry<Req, Resp>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<Resp, OperatorError>
    where
        Req: Serialize + ?Sized,
        Resp: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut last_error = String::new();

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay_ms = BACKOFF_DELAYS_MS[attempt - 1];
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }

            let result = self.client.post(&url).json(body).send().await;

            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        return response.json::<Resp>().await.map_err(|e| {
                            OperatorError::ParseError(format!(
                                "failed to parse response: {e}"
                            ))
                        });
                    }
                    // Non-200 status — treat as failure, retry
                    last_error = format!(
                        "non-200 status {} from {path}",
                        response.status()
                    );
                    tracing::warn!(
                        attempt = attempt + 1,
                        status = %response.status(),
                        "operator REST call failed, will retry"
                    );
                }
                Err(e) => {
                    last_error = format!("HTTP error: {e}");
                    tracing::warn!(
                        attempt = attempt + 1,
                        error = %e,
                        "operator REST call failed, will retry"
                    );
                }
            }
        }

        Err(OperatorError::RetriesExhausted(last_error))
    }
}

impl ParkingOperator for OperatorClient {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        let request = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp: chrono_timestamp(),
        };
        self.post_with_retry("/parking/start", &request).await
    }

    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        let request = StopRequest {
            session_id: session_id.to_string(),
            timestamp: chrono_timestamp(),
        };
        self.post_with_retry("/parking/stop", &request).await
    }
}

/// Returns the current Unix timestamp in seconds.
fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // TS-08-8: Operator Start Session REST Call
    #[tokio::test]
    async fn test_start_session_request() {
        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
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
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
        assert!((resp.rate.amount - 2.5).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");
    }

    // TS-08-9: Operator Stop Session REST Call
    #[tokio::test]
    async fn test_stop_session_request() {
        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "session_id": "sess-1",
            "status": "completed",
            "duration_seconds": 3600,
            "total_amount": 2.50,
            "currency": "EUR"
        });

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
    #[tokio::test]
    async fn test_start_response_parsing() {
        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
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
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await.unwrap();
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.5).abs() < f64::EPSILON);
    }

    // TS-08-E3: Operator REST Retry on Failure
    #[tokio::test]
    async fn test_retry_on_failure() {
        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "session_id": "s1",
            "status": "active",
            "rate": {
                "type": "per_hour",
                "amount": 2.5,
                "currency": "EUR"
            }
        });

        // First 2 requests fail with 500, third succeeds.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_ok(), "should succeed after retries");
    }

    // TS-08-E4: Operator REST All Retries Exhausted
    #[tokio::test]
    async fn test_retry_exhausted() {
        let mock_server = MockServer::start().await;

        // All requests fail
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_err(), "should fail after all retries exhausted");
    }

    // TS-08-E5: Operator Non-200 Status Triggers Retry
    #[tokio::test]
    async fn test_retry_on_non_200() {
        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "session_id": "s1",
            "status": "active",
            "rate": {
                "type": "per_hour",
                "amount": 2.5,
                "currency": "EUR"
            }
        });

        // 500 twice, then 200
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;
        assert!(resp.is_ok(), "should succeed after non-200 retries");
    }
}
