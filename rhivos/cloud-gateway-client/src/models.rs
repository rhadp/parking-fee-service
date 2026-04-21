use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Inbound lock/unlock command received from NATS and forwarded to DATA_BROKER.
///
/// `doors` uses `Vec<Value>` rather than `Vec<String>` so that individual door
/// values are not type-validated here — that responsibility belongs to
/// LOCKING_SERVICE (REQ-6.4).
#[derive(Debug, Deserialize)]
pub struct CommandPayload {
    pub command_id: String,
    pub action: String, // "lock" or "unlock"
    pub doors: Vec<Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Command result relayed verbatim from DATA_BROKER to NATS.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String, // "success" or "failed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: u64,
}

/// Aggregated vehicle telemetry published to NATS on every signal change.
///
/// Fields are `Option<_>` so that signals never set in DATA_BROKER are
/// omitted from the JSON payload (REQ-8.3).
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

/// Self-registration message published to `vehicles.{VIN}.status` on startup.
#[derive(Debug, Serialize)]
pub struct RegistrationMessage {
    pub vin: String,
    pub status: String, // always "online"
    pub timestamp: u64,
}

/// Signal update received from DATA_BROKER telemetry subscription.
#[derive(Debug, Clone)]
pub enum SignalUpdate {
    IsLocked(bool),
    Latitude(f64),
    Longitude(f64),
    ParkingActive(bool),
}

// ─────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // TS-04-P1: Registration message format
    // Validates: [04-REQ-4.1]
    #[test]
    fn ts_04_p1_registration_message_format() {
        let msg = RegistrationMessage {
            vin: "VIN-001".to_string(),
            status: "online".to_string(),
            timestamp: 1700000000,
        };
        let json = serde_json::to_string(&msg).expect("serialization must succeed");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("must parse back to JSON");

        assert_eq!(parsed["vin"], "VIN-001");
        assert_eq!(parsed["status"], "online");
        assert!(parsed["timestamp"].is_number(), "timestamp must be a number");
    }
}
