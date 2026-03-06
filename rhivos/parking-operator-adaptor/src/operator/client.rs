//! REST client for communicating with the PARKING_OPERATOR.
//!
//! Implements `POST /parking/start`, `POST /parking/stop`, and
//! `GET /parking/status/{session_id}` with a 5-second request timeout.

use std::time::Duration;

use reqwest::Client;
use tracing::{error, info};

use super::models::*;

/// HTTP request timeout (5 seconds per 08-REQ-7.E1).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Error type for operator REST client operations.
#[derive(Debug)]
pub enum OperatorError {
    /// Connection refused or DNS failure.
    Unreachable(String),
    /// Request exceeded the 5-second timeout.
    Timeout,
    /// Non-200 HTTP response with status code and body.
    HttpError(u16, String),
    /// Failed to parse the JSON response body.
    ParseError(String),
}

impl std::fmt::Display for OperatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorError::Unreachable(msg) => write!(f, "operator unreachable: {msg}"),
            OperatorError::Timeout => write!(f, "operator request timed out"),
            OperatorError::HttpError(code, body) => {
                write!(f, "operator returned HTTP {code}: {body}")
            }
            OperatorError::ParseError(msg) => write!(f, "failed to parse response: {msg}"),
        }
    }
}

impl std::error::Error for OperatorError {}

/// REST client for communicating with the PARKING_OPERATOR.
pub struct OperatorClient {
    base_url: String,
    client: Client,
}

impl OperatorClient {
    /// Creates a new OperatorClient with the given base URL and a 5-second timeout.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to create HTTP client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Sends `POST /parking/start` to the operator.
    ///
    /// Requirements: 08-REQ-7.1
    pub async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        let url = format!("{}/parking/start", self.base_url);
        let timestamp = chrono::Utc::now().timestamp();

        let body = StartRequest {
            vehicle_id: vehicle_id.to_string(),
            zone_id: zone_id.to_string(),
            timestamp,
        };

        info!(%url, %vehicle_id, %zone_id, "POST /parking/start");

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    OperatorError::Timeout
                } else {
                    OperatorError::Unreachable(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            error!(http_status = %status, body = %body_text, "operator start failed");
            return Err(OperatorError::HttpError(status.as_u16(), body_text));
        }

        response
            .json::<StartResponse>()
            .await
            .map_err(|e| OperatorError::ParseError(e.to_string()))
    }

    /// Sends `POST /parking/stop` to the operator.
    ///
    /// Requirements: 08-REQ-7.2
    pub async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        let url = format!("{}/parking/stop", self.base_url);
        let timestamp = chrono::Utc::now().timestamp();

        let body = StopRequest {
            session_id: session_id.to_string(),
            timestamp,
        };

        info!(%url, %session_id, "POST /parking/stop");

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    OperatorError::Timeout
                } else {
                    OperatorError::Unreachable(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            error!(http_status = %status, body = %body_text, "operator stop failed");
            return Err(OperatorError::HttpError(status.as_u16(), body_text));
        }

        response
            .json::<StopResponse>()
            .await
            .map_err(|e| OperatorError::ParseError(e.to_string()))
    }

    /// Sends `GET /parking/status/{session_id}` to the operator.
    ///
    /// Requirements: 08-REQ-7.3
    pub async fn get_status(&self, session_id: &str) -> Result<StatusResponse, OperatorError> {
        let url = format!("{}/parking/status/{}", self.base_url, session_id);

        info!(%url, %session_id, "GET /parking/status");

        let response = self.client.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                OperatorError::Timeout
            } else {
                OperatorError::Unreachable(e.to_string())
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            error!(http_status = %status, body = %body_text, "operator status query failed");
            return Err(OperatorError::HttpError(status.as_u16(), body_text));
        }

        response
            .json::<StatusResponse>()
            .await
            .map_err(|e| OperatorError::ParseError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify OperatorClient can be created with a base URL.
    #[test]
    fn test_client_new() {
        let client = OperatorClient::new("http://localhost:8080");
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    /// Verify trailing slashes are stripped from the base URL.
    #[test]
    fn test_client_strips_trailing_slash() {
        let client = OperatorClient::new("http://localhost:8080/");
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    /// Verify that calling start_session on unreachable host returns Unreachable error.
    #[tokio::test]
    async fn test_start_session_unreachable() {
        // Use a port that's almost certainly not listening
        let client = OperatorClient::new("http://127.0.0.1:19999");
        let result = client.start_session("VIN-001", "zone-1").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OperatorError::Unreachable(_) => {} // expected
            other => panic!("expected Unreachable, got: {other:?}"),
        }
    }

    /// Verify that calling stop_session on unreachable host returns Unreachable error.
    #[tokio::test]
    async fn test_stop_session_unreachable() {
        let client = OperatorClient::new("http://127.0.0.1:19999");
        let result = client.stop_session("sess-abc").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OperatorError::Unreachable(_) => {}
            other => panic!("expected Unreachable, got: {other:?}"),
        }
    }

    /// Verify that calling get_status on unreachable host returns Unreachable error.
    #[tokio::test]
    async fn test_get_status_unreachable() {
        let client = OperatorClient::new("http://127.0.0.1:19999");
        let result = client.get_status("sess-abc").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OperatorError::Unreachable(_) => {}
            other => panic!("expected Unreachable, got: {other:?}"),
        }
    }
}
