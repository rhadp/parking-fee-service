#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// A lock/unlock command received from NATS.
///
/// `doors` uses `serde_json::Value` to accept any JSON value without enforcing
/// element types — individual door values are intentionally not validated
/// here (REQ-6.4; LOCKING_SERVICE owns that responsibility).
#[derive(Debug, Deserialize)]
pub struct CommandPayload {
    pub command_id: String,
    pub action: String,
    pub doors: Vec<serde_json::Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Result of a lock/unlock command, observed from DATA_BROKER.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: u64,
}

/// Aggregated vehicle telemetry, published to NATS on signal change.
///
/// Fields are `Option` so that signals that have never been observed can be
/// omitted from the serialized JSON (REQ-8.3).
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

/// Self-registration announcement published to `vehicles.{VIN}.status` on startup.
#[derive(Debug, Serialize)]
pub struct RegistrationMessage {
    pub vin: String,
    pub status: String,
    pub timestamp: u64,
}

/// A change event from a DATA_BROKER telemetry signal subscription.
#[derive(Debug)]
pub enum SignalUpdate {
    IsLocked(bool),
    Latitude(f64),
    Longitude(f64),
    ParkingActive(bool),
}
