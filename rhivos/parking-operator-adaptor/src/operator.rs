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

/// HTTP-backed PARKING_OPERATOR REST client.
pub struct OperatorClient {
    _client: reqwest::Client,
    _base_url: String,
}

impl OperatorClient {
    /// Create a new OperatorClient targeting the given base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            _client: reqwest::Client::new(),
            _base_url: base_url.to_string(),
        }
    }
}

impl ParkingOperator for OperatorClient {
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
