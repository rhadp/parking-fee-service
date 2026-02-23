//! REST client for the PARKING_OPERATOR service.
//!
//! Provides async methods to interact with the PARKING_OPERATOR REST API
//! for starting/stopping parking sessions, querying session status, and
//! retrieving zone rate information.

use serde::{Deserialize, Serialize};

/// Error type for operator client operations.
#[derive(Debug)]
pub enum OperatorError {
    /// The PARKING_OPERATOR service is unreachable.
    Unreachable(String),
    /// The requested resource was not found (HTTP 404).
    NotFound(String),
    /// An unexpected error occurred.
    Other(String),
}

impl std::fmt::Display for OperatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorError::Unreachable(msg) => write!(f, "operator unreachable: {}", msg),
            OperatorError::NotFound(msg) => write!(f, "not found: {}", msg),
            OperatorError::Other(msg) => write!(f, "operator error: {}", msg),
        }
    }
}

impl std::error::Error for OperatorError {}

// ---- Request/Response types ----

/// Request body for POST /parking/start.
#[derive(Debug, Serialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub zone_id: String,
    pub timestamp: i64,
}

/// Response body from POST /parking/start.
#[derive(Debug, Deserialize)]
pub struct StartResponse {
    pub session_id: String,
    pub status: String,
}

/// Request body for POST /parking/stop.
#[derive(Debug, Serialize)]
pub struct StopRequest {
    pub session_id: String,
}

/// Response body from POST /parking/stop.
#[derive(Debug, Deserialize)]
pub struct StopResponse {
    pub session_id: String,
    pub fee: f64,
    pub duration_seconds: i64,
    pub currency: String,
}

/// Response body from GET /parking/{session_id}/status.
#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    pub session_id: String,
    pub active: bool,
    pub start_time: i64,
    pub current_fee: f64,
    pub currency: String,
}

/// Response body from GET /rate/{zone_id}.
#[derive(Debug, Deserialize)]
pub struct RateResponse {
    pub rate_per_hour: f64,
    pub currency: String,
    pub zone_name: String,
}

/// Client for the PARKING_OPERATOR REST API.
#[derive(Debug, Clone)]
pub struct OperatorClient {
    base_url: String,
    http: reqwest::Client,
}

impl OperatorClient {
    /// Create a new operator client with the given base URL.
    ///
    /// The base URL should include the scheme (e.g., `http://localhost:8090`).
    pub fn new(base_url: &str) -> Self {
        OperatorClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Start a parking session.
    ///
    /// Calls `POST /parking/start` on the PARKING_OPERATOR.
    pub async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
        timestamp: i64,
    ) -> Result<StartResponse, OperatorError> {
        let url = format!("{}/parking/start", self.base_url);
        let body = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp,
        };

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OperatorError::Unreachable(e.to_string()))?;

        if resp.status().is_success() {
            resp.json::<StartResponse>()
                .await
                .map_err(|e| OperatorError::Other(e.to_string()))
        } else {
            Err(OperatorError::Other(format!(
                "unexpected status: {}",
                resp.status()
            )))
        }
    }

    /// Stop a parking session.
    ///
    /// Calls `POST /parking/stop` on the PARKING_OPERATOR.
    pub async fn stop_session(
        &self,
        session_id: &str,
    ) -> Result<StopResponse, OperatorError> {
        let url = format!("{}/parking/stop", self.base_url);
        let body = StopRequest {
            session_id: session_id.to_string(),
        };

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OperatorError::Unreachable(e.to_string()))?;

        match resp.status().as_u16() {
            200 => resp
                .json::<StopResponse>()
                .await
                .map_err(|e| OperatorError::Other(e.to_string())),
            404 => Err(OperatorError::NotFound(format!(
                "session {} not found",
                session_id
            ))),
            status => Err(OperatorError::Other(format!(
                "unexpected status: {}",
                status
            ))),
        }
    }

    /// Get the status of a parking session.
    ///
    /// Calls `GET /parking/{session_id}/status` on the PARKING_OPERATOR.
    pub async fn get_status(
        &self,
        session_id: &str,
    ) -> Result<StatusResponse, OperatorError> {
        let url = format!("{}/parking/{}/status", self.base_url, session_id);

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| OperatorError::Unreachable(e.to_string()))?;

        match resp.status().as_u16() {
            200 => resp
                .json::<StatusResponse>()
                .await
                .map_err(|e| OperatorError::Other(e.to_string())),
            404 => Err(OperatorError::NotFound(format!(
                "session {} not found",
                session_id
            ))),
            status => Err(OperatorError::Other(format!(
                "unexpected status: {}",
                status
            ))),
        }
    }

    /// Get the rate for a parking zone.
    ///
    /// Calls `GET /rate/{zone_id}` on the PARKING_OPERATOR.
    pub async fn get_rate(&self, zone_id: &str) -> Result<RateResponse, OperatorError> {
        let url = format!("{}/rate/{}", self.base_url, zone_id);

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| OperatorError::Unreachable(e.to_string()))?;

        match resp.status().as_u16() {
            200 => resp
                .json::<RateResponse>()
                .await
                .map_err(|e| OperatorError::Other(e.to_string())),
            404 => Err(OperatorError::NotFound(format!(
                "zone {} not found",
                zone_id
            ))),
            status => Err(OperatorError::Other(format!(
                "unexpected status: {}",
                status
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_client_creation() {
        let client = OperatorClient::new("http://localhost:8090");
        assert_eq!(client.base_url, "http://localhost:8090");
    }

    #[test]
    fn test_operator_client_strips_trailing_slash() {
        let client = OperatorClient::new("http://localhost:8090/");
        assert_eq!(client.base_url, "http://localhost:8090");
    }

    #[test]
    fn test_operator_error_display() {
        let err = OperatorError::Unreachable("connection refused".to_string());
        assert!(err.to_string().contains("unreachable"));
        assert!(err.to_string().contains("connection refused"));

        let err = OperatorError::NotFound("session 123".to_string());
        assert!(err.to_string().contains("not found"));

        let err = OperatorError::Other("something went wrong".to_string());
        assert!(err.to_string().contains("something went wrong"));
    }
}
