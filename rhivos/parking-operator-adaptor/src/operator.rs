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
/// outbound REST calls. Total of 4 attempts (1 initial + 3 retries).
pub struct OperatorClient {
    base_url: String,
    client: reqwest::Client,
}

/// Delay (in seconds) between retry attempts: 1 s, 2 s, 4 s.
const RETRY_DELAYS_SECS: &[u64] = &[1, 2, 4];

impl OperatorClient {
    /// Create a new client targeting `base_url`.
    pub fn new(base_url: &str) -> Self {
        OperatorClient {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Execute a POST request to `path` with `body`, retrying up to 3 times
    /// with exponential backoff (1 s, 2 s, 4 s) on failure or non-200 status.
    async fn post_with_retry<Req, Resp>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<Resp, OperatorError>
    where
        Req: serde::Serialize,
        Resp: for<'de> serde::Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut last_err = OperatorError::Unavailable("no attempts made".to_string());

        // 1 initial attempt + 3 retries = 4 total attempts
        let delays: Vec<Option<u64>> = std::iter::once(None)
            .chain(RETRY_DELAYS_SECS.iter().copied().map(Some))
            .collect();

        for (attempt, delay_opt) in delays.iter().enumerate() {
            if let Some(secs) = delay_opt {
                tokio::time::sleep(tokio::time::Duration::from_secs(*secs)).await;
            }

            match self.client.post(&url).json(body).send().await {
                Ok(resp) if resp.status().is_success() => match resp.json::<Resp>().await {
                    Ok(parsed) => return Ok(parsed),
                    Err(e) => {
                        last_err =
                            OperatorError::Unavailable(format!("response parse error: {e}"));
                        tracing::warn!(attempt = attempt + 1, %last_err, "parse failed");
                    }
                },
                Ok(resp) => {
                    let status = resp.status();
                    last_err = OperatorError::Unavailable(format!("HTTP {status}"));
                    tracing::warn!(attempt = attempt + 1, %last_err, "non-2xx response");
                }
                Err(e) => {
                    last_err = OperatorError::Unavailable(format!("request error: {e}"));
                    tracing::warn!(attempt = attempt + 1, %last_err, "request failed");
                }
            }
        }

        Err(last_err)
    }
}

#[async_trait]
impl OperatorApi for OperatorClient {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        let body = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        };
        self.post_with_retry("/parking/start", &body).await
    }

    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        let body = StopRequest {
            session_id: session_id.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        };
        self.post_with_retry("/parking/stop", &body).await
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

        // First two requests → 500, third → 200.
        // .expect(2) verifies exactly 2 failure responses were served.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        // .expect(1) verifies exactly 1 success response was served.
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

        assert!(resp.is_ok(), "expected Ok after retries succeeded");
        assert_eq!(resp.unwrap().session_id, "s1");
        // Wiremock verifies exact request counts (2 failures + 1 success = 3 total) at drop.
    }

    /// TS-08-E4: All retries exhausted → OperatorError returned.
    ///
    /// Verifies: 08-REQ-2.E1
    /// Asserts exactly 4 HTTP requests (1 initial + 3 retries) per test spec.
    #[tokio::test]
    async fn test_retry_exhausted() {
        let mock_server = MockServer::start().await;

        // .expect(4) verifies 4 total requests: 1 initial + 3 retries.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .expect(4)
            .mount(&mock_server)
            .await;

        let client = OperatorClient::new(&mock_server.uri());
        let resp = client.start_session("DEMO-VIN-001", "zone-a").await;

        assert!(resp.is_err(), "expected Err after all retries exhausted");
        assert!(
            matches!(resp.unwrap_err(), OperatorError::Unavailable(_)),
            "expected OperatorError::Unavailable"
        );
        // Wiremock verifies exactly 4 requests at drop.
    }

    /// TS-08-E5: Non-200 HTTP status triggers retry logic.
    ///
    /// Verifies: 08-REQ-2.E2
    #[tokio::test]
    async fn test_retry_on_non_200() {
        let mock_server = MockServer::start().await;

        // Two 500s, then success.
        // .expect(2) verifies exactly 2 non-200 responses were served.
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        // .expect(1) verifies exactly 1 success response was served.
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

        assert!(resp.is_ok(), "expected Ok after non-200 retries succeeded");
        // Wiremock verifies exact request counts (2 failures + 1 success = 3 total) at drop.
    }
}
