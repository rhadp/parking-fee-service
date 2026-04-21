//! PARKING_OPERATOR REST client.
//!
//! Sends `POST /parking/start` and `POST /parking/stop` requests to the
//! external parking operator backend, parsing JSON responses into typed structs.
//! Retry logic applies exponential backoff: 1 s, 2 s, 4 s (up to 3 retries).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::session::Rate;

/// Response from `POST /parking/start`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
    pub rate: Rate,
}

/// Response from `POST /parking/stop`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StopResponse {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: u64,
    pub total_amount: f64,
    pub currency: String,
}

/// Errors returned by the PARKING_OPERATOR REST client.
#[derive(Debug, Clone)]
pub enum OperatorError {
    /// The operator was unreachable after all retries.
    Unavailable(String),
}

impl std::fmt::Display for OperatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorError::Unavailable(msg) => write!(f, "operator unavailable: {msg}"),
        }
    }
}

/// Trait abstracting the PARKING_OPERATOR REST API.
///
/// Implementations must be `Send + Sync` so they can be shared across async
/// task boundaries and used as trait objects (`&dyn OperatorApi`).
#[async_trait]
pub trait OperatorApi: Send + Sync {
    /// Start a parking session with the operator.
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError>;

    /// Stop the active parking session with the operator.
    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError>;
}

/// Concrete PARKING_OPERATOR REST client backed by `reqwest`.
///
/// Implements retry with exponential backoff (1 s, 2 s, 4 s) for all
/// outbound REST calls.
#[allow(dead_code)] // fields used once implementation is complete (task group 3)
pub struct OperatorClient {
    base_url: String,
    client: reqwest::Client,
}

impl OperatorClient {
    /// Create a new client targeting `base_url`.
    pub fn new(base_url: &str) -> Self {
        OperatorClient {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl OperatorApi for OperatorClient {
    async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        todo!("implement OperatorClient::start_session")
    }

    async fn stop_session(&self, _session_id: &str) -> Result<StopResponse, OperatorError> {
        todo!("implement OperatorClient::stop_session")
    }
}

// ─── Request body types (used when sending JSON to the operator) ─────────────

/// Request body for `POST /parking/start`.
#[derive(Debug, Serialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub zone_id: String,
    /// Unix timestamp (seconds).
    pub timestamp: i64,
}

/// Request body for `POST /parking/stop`.
#[derive(Debug, Serialize)]
pub struct StopRequest {
    pub session_id: String,
    /// Unix timestamp (seconds).
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn start_response_json() -> serde_json::Value {
        json!({
            "session_id": "s1",
            "status": "active",
            "rate": {
                "type": "per_hour",
                "amount": 2.50,
                "currency": "EUR"
            }
        })
    }

    fn stop_response_json() -> serde_json::Value {
        json!({
            "session_id": "sess-1",
            "status": "completed",
            "duration_seconds": 3600,
            "total_amount": 2.50,
            "currency": "EUR"
        })
    }

    /// TS-08-8: Correct POST /parking/start request is sent.
    ///
    /// Verifies: 08-REQ-2.1
    #[tokio::test]
    async fn test_start_session_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(start_response_json()),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("DEMO-VIN-001", "zone-a").await;

        assert!(resp.is_ok(), "expected Ok from start_session");
        let resp = resp.unwrap();
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
    }

    /// TS-08-9: Correct POST /parking/stop request is sent.
    ///
    /// Verifies: 08-REQ-2.2, 08-REQ-2.4
    #[tokio::test]
    async fn test_stop_session_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(stop_response_json()),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.stop_session("sess-1").await;

        assert!(resp.is_ok(), "expected Ok from stop_session");
        let resp = resp.unwrap();
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert!((resp.total_amount - 2.50).abs() < f64::EPSILON);
    }

    /// TS-08-10: Start response is parsed into StartResponse with rate.
    ///
    /// Verifies: 08-REQ-2.3
    #[tokio::test]
    async fn test_start_response_parsing() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(start_response_json()),
            )
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("DEMO-VIN-001", "zone-a").await;

        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.rate.rate_type, "per_hour");
        assert!((resp.rate.amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.rate.currency, "EUR");
    }

    /// TS-08-E3: Operator is retried when the first two requests fail.
    ///
    /// Verifies: 08-REQ-2.E1
    #[tokio::test]
    async fn test_retry_on_failure() {
        let mock_server = MockServer::start().await;

        // First two requests → 500, third → 200
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(start_response_json()),
            )
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("DEMO-VIN-001", "zone-a").await;

        assert!(resp.is_ok(), "expected Ok after retries succeeded");
        assert_eq!(resp.unwrap().session_id, "s1");
        // Wiremock verifies that exactly 3 requests were received at scope drop.
    }

    /// TS-08-E4: All retries exhausted → OperatorError returned.
    ///
    /// Verifies: 08-REQ-2.E1
    #[tokio::test]
    async fn test_retry_exhausted() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("DEMO-VIN-001", "zone-a").await;

        assert!(resp.is_err(), "expected Err after all retries exhausted");
        assert!(
            matches!(resp.unwrap_err(), OperatorError::Unavailable(_)),
            "expected OperatorError::Unavailable"
        );
    }

    /// TS-08-E5: Non-200 HTTP status triggers retry logic.
    ///
    /// Verifies: 08-REQ-2.E2
    #[tokio::test]
    async fn test_retry_on_non_200() {
        let mock_server = MockServer::start().await;

        // Two 500s, then success
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(start_response_json()),
            )
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("DEMO-VIN-001", "zone-a").await;

        assert!(resp.is_ok(), "expected Ok after non-200 retries succeeded");
    }
}
