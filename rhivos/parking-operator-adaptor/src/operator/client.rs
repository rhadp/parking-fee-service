use super::models::*;

/// Error type for operator REST client operations.
#[derive(Debug)]
pub enum OperatorError {
    Unreachable(String),
    Timeout,
    HttpError(u16, String),
    ParseError(String),
}

/// REST client for communicating with the PARKING_OPERATOR.
/// Stub: will be implemented in task group 4.
pub struct OperatorClient {
    _base_url: String,
}

impl OperatorClient {
    /// Creates a new OperatorClient with the given base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            _base_url: base_url.to_string(),
        }
    }

    /// Sends POST /parking/start to the operator.
    pub async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        // Stub: not yet implemented
        Err(OperatorError::Unreachable("not implemented".to_string()))
    }

    /// Sends POST /parking/stop to the operator.
    pub async fn stop_session(
        &self,
        _session_id: &str,
    ) -> Result<StopResponse, OperatorError> {
        // Stub: not yet implemented
        Err(OperatorError::Unreachable("not implemented".to_string()))
    }

    /// Sends GET /parking/status/{session_id} to the operator.
    pub async fn get_status(
        &self,
        _session_id: &str,
    ) -> Result<StatusResponse, OperatorError> {
        // Stub: not yet implemented
        Err(OperatorError::Unreachable("not implemented".to_string()))
    }
}
