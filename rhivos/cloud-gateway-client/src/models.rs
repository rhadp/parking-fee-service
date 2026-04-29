use serde::{Deserialize, Serialize};

/// A validated lock/unlock command received from the cloud.
#[derive(Debug, Deserialize)]
pub struct CommandPayload {
    pub command_id: String,
    pub action: String,
    pub doors: Vec<String>,
    /// Extra fields are captured to support passthrough fidelity.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A command result observed from DATA_BROKER.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: u64,
}

/// Aggregated telemetry state published to NATS.
#[derive(Debug, Serialize)]
pub struct TelemetryMessage {
    pub vin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parking_active: Option<bool>,
    pub timestamp: u64,
}

/// Self-registration message published on startup.
#[derive(Debug, Serialize)]
pub struct RegistrationMessage {
    pub vin: String,
    pub status: String,
    pub timestamp: u64,
}

/// A single signal update from DATA_BROKER telemetry subscription.
#[derive(Debug, Clone)]
pub enum SignalUpdate {
    IsLocked(bool),
    Latitude(f64),
    Longitude(f64),
    ParkingActive(bool),
}
