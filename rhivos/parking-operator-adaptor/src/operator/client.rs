//! HTTP client for the PARKING_OPERATOR REST API.
//!
//! [`OperatorClient`] implements [`super::OperatorApi`] using `reqwest`.
//!
//! Requirement: 08-REQ-1.1, 08-REQ-2.1

use super::models::{StartRequest, StartResponse, StopRequest, StopResponse};
use super::OperatorApi;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by [`OperatorClient`].
#[derive(Debug, Clone)]
pub enum OperatorError {
    /// The PARKING_OPERATOR is unreachable or returned a network error.
    Unreachable(String),
    /// The PARKING_OPERATOR returned a non-success HTTP status.
    ServerError(u16, String),
    /// Response body could not be parsed.
    ParseError(String),
}

impl std::fmt::Display for OperatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(msg) => write!(f, "operator unreachable: {msg}"),
            Self::ServerError(code, msg) => write!(f, "operator error {code}: {msg}"),
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for OperatorError {}

// ---------------------------------------------------------------------------
// HTTP client
// ---------------------------------------------------------------------------

/// Concrete HTTP client for the PARKING_OPERATOR REST API.
///
/// Uses `reqwest` to make JSON HTTP requests to the PARKING_OPERATOR backend.
/// `reqwest::Client` is internally `Arc`-based and can be cheaply cloned.
pub struct OperatorClient {
    base_url: String,
    http: reqwest::Client,
}

impl OperatorClient {
    /// Create a new `OperatorClient` for the given base URL.
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http: reqwest::Client::new(),
        }
    }
}

#[tonic::async_trait]
impl OperatorApi for OperatorClient {
    /// Call `POST /parking/start` with vehicle_id, zone_id, and current timestamp.
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let body = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp,
        };

        let url = format!("{}/parking/start", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OperatorError::Unreachable(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            let msg = resp.text().await.unwrap_or_default();
            return Err(OperatorError::ServerError(code, msg));
        }

        resp.json::<StartResponse>()
            .await
            .map_err(|e| OperatorError::ParseError(e.to_string()))
    }

    /// Call `POST /parking/stop` with session_id and current timestamp.
    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let body = StopRequest {
            session_id: session_id.to_string(),
            timestamp,
        };

        let url = format!("{}/parking/stop", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OperatorError::Unreachable(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            let msg = resp.text().await.unwrap_or_default();
            return Err(OperatorError::ServerError(code, msg));
        }

        resp.json::<StopResponse>()
            .await
            .map_err(|e| OperatorError::ParseError(e.to_string()))
    }
}
