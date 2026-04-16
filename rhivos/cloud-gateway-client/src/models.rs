use serde::{Deserialize, Serialize};

/// A lock/unlock command payload received from NATS.
///
/// NOTE: `doors` uses `Vec<serde_json::Value>` rather than `Vec<String>` to
/// avoid implicitly validating the type of individual door elements —
/// REQ-6.4 explicitly assigns that responsibility to LOCKING_SERVICE.
#[derive(Debug, Deserialize)]
pub struct CommandPayload {
    pub command_id: String,
    pub action: String,
    pub doors: Vec<serde_json::Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Command response observed from DATA_BROKER and relayed to NATS.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: u64,
}

/// Aggregated vehicle telemetry published to NATS.
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
    pub status: String,
    pub timestamp: u64,
}

impl RegistrationMessage {
    /// Create a new registration message for the given VIN.
    /// The timestamp is the current Unix epoch in seconds.
    #[allow(unused_variables)]
    pub fn new(vin: impl Into<String>) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        RegistrationMessage {
            vin: vin.into(),
            status: "online".to_string(),
            timestamp,
        }
    }
}

/// A signal update received from DATA_BROKER.
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
    fn ts_04_p1_registration_message_format() {
        let msg = RegistrationMessage::new("VIN-001");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(
            json.contains(r#""vin":"VIN-001""#),
            "registration JSON must contain vin field: {}",
            json
        );
        assert!(
            json.contains(r#""status":"online""#),
            "registration JSON must contain status:online: {}",
            json
        );
        assert!(
            json.contains(r#""timestamp""#),
            "registration JSON must contain timestamp field: {}",
            json
        );
    }
}
