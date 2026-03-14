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
/// # Stub
/// Not yet implemented — task group 4 adds the real `reqwest` calls.
pub struct OperatorClient {
    _base_url: String,
    _http: reqwest::Client,
}

impl OperatorClient {
    /// Create a new `OperatorClient` for the given base URL.
    pub fn new(base_url: String) -> Self {
        Self {
            _base_url: base_url,
            _http: reqwest::Client::new(),
        }
    }
}

#[tonic::async_trait]
impl OperatorApi for OperatorClient {
    async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        // STUB: task group 4 implements real POST /parking/start.
        Err(OperatorError::Unreachable("not implemented".to_string()))
    }

    async fn stop_session(&self, _session_id: &str) -> Result<StopResponse, OperatorError> {
        // STUB: task group 4 implements real POST /parking/stop.
        Err(OperatorError::Unreachable("not implemented".to_string()))
    }
}

// Suppress unused import warnings on the models in the stub.
const _: fn() = || {
    let _ = std::mem::size_of::<StartRequest>();
    let _ = std::mem::size_of::<StopRequest>();
};
