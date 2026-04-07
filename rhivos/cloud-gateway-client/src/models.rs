/// Data model types for the cloud-gateway-client service.
use serde::{Deserialize, Serialize};

/// A validated command payload received from NATS.
#[derive(Debug, Deserialize)]
pub struct CommandPayload {
    pub command_id: String,
    pub action: String,
    pub doors: Vec<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A command response from DATA_BROKER to relay to NATS.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: u64,
}

/// Aggregated telemetry message published to NATS.
#[derive(Debug, Serialize, Deserialize)]
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

/// Registration message published on startup.
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationMessage {
    pub vin: String,
    pub status: String,
    pub timestamp: u64,
}

/// A signal update from DATA_BROKER for telemetry aggregation.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalUpdate {
    IsLocked(bool),
    Latitude(f64),
    Longitude(f64),
    ParkingActive(bool),
}
