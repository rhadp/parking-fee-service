//! Data models for the cloud-gateway-client service.

use serde::{Deserialize, Serialize};

/// A lock/unlock command received from NATS.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct CommandPayload {
    pub command_id: String,
    pub action: String,
    pub doors: Vec<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A command response from LOCKING_SERVICE via DATA_BROKER.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: u64,
}

/// Aggregated telemetry message published to NATS.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RegistrationMessage {
    pub vin: String,
    pub status: String,
    pub timestamp: u64,
}

/// A signal update from DATA_BROKER telemetry subscription.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalUpdate {
    IsLocked(bool),
    Latitude(f64),
    Longitude(f64),
    ParkingActive(bool),
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-04-P1: Registration message format
    #[test]
    fn test_registration_message_format() {
        let ts = 1_700_000_000u64;
        let msg = RegistrationMessage {
            vin: "VIN-001".to_string(),
            status: "online".to_string(),
            timestamp: ts,
        };
        let json = serde_json::to_string(&msg).expect("should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("should parse as JSON");

        assert_eq!(parsed["vin"], "VIN-001");
        assert_eq!(parsed["status"], "online");
        assert!(parsed.get("timestamp").is_some());
        assert_eq!(parsed["timestamp"], ts);
    }
}
