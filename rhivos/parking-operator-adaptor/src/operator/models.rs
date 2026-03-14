//! Data models for the PARKING_OPERATOR REST API.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Body for `POST /parking/start`.
#[derive(Debug, Clone, Serialize)]
pub struct StartRequest {
    /// Vehicle identifier.
    pub vehicle_id: String,
    /// Parking zone identifier.
    pub zone_id: String,
    /// Unix timestamp of the start request.
    pub timestamp: i64,
}

/// Body for `POST /parking/stop`.
#[derive(Debug, Clone, Serialize)]
pub struct StopRequest {
    /// The session identifier returned by `/parking/start`.
    pub session_id: String,
    /// Unix timestamp of the stop request.
    pub timestamp: i64,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response from `POST /parking/start`.
#[derive(Debug, Clone, Deserialize)]
pub struct StartResponse {
    /// Operator-assigned session identifier.
    pub session_id: String,
    /// Session status string (e.g. `"active"`).
    pub status: String,
    /// Pricing information for the session (optional — older operators may omit).
    pub rate: Option<crate::session::Rate>,
}

/// Response from `POST /parking/stop`.
#[derive(Debug, Clone, Deserialize)]
pub struct StopResponse {
    /// The session that was stopped.
    pub session_id: String,
    /// Session duration in seconds.
    pub duration: u64,
    /// Total parking fee.
    pub fee: f64,
    /// Status string (e.g. `"completed"`).
    pub status: String,
}
