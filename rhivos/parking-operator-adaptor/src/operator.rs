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
#[allow(dead_code)]
pub struct OperatorClient {
    client: reqwest::Client,
    base_url: String,
}

impl OperatorClient {
    /// Create a new operator client with the given base URL.
    pub fn new(_base_url: &str) -> Self {
        todo!()
    }
}

impl ParkingOperator for OperatorClient {
    async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        todo!()
    }

    async fn stop_session(&self, _session_id: &str) -> Result<StopResponse, OperatorError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
            .mount(&mock_server)
            .await;

        // Third request returns 200
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
        let resp = client.start_session("VIN", "zone").await;

        // THEN the call succeeds after retries
        assert!(resp.is_ok());
        assert_eq!(resp.unwrap().session_id, "s1");
    }

    // TS-08-E4: Operator REST All Retries Exhausted
    // Validates: [08-REQ-2.E1]
    #[tokio::test]
    async fn test_retry_exhausted() {
        // GIVEN a mock HTTP server that always fails
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // WHEN start_session is called
        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;

        // THEN error is returned after all retries
        assert!(resp.is_err());
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
            .mount(&mock_server)
            .await;

        // WHEN start_session is called
        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("VIN", "zone").await;

        // THEN the call succeeds after retries
        assert!(resp.is_ok());
    }
}
