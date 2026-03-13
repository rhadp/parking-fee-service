use serde::Serialize;

/// Aggregated state of telemetry signals from DATA_BROKER.
/// Fields are `None` when the signal has never been set.
#[derive(Debug, Clone, Default)]
pub struct TelemetryState {
    pub is_locked: Option<bool>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub parking_active: Option<bool>,
}

/// Aggregated telemetry message published to NATS.
/// Optional fields are omitted from JSON when `None`.
#[derive(Debug, Clone, Serialize)]
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
    pub timestamp: i64,
}

/// Build an aggregated telemetry message from the current state.
///
/// Includes the VIN, all `Some` signal values, and the current Unix timestamp.
/// `None` fields are omitted from the serialized JSON (04-REQ-4.E1).
pub fn build_telemetry(_vin: &str, _state: &TelemetryState) -> TelemetryMessage {
    todo!("build_telemetry: create TelemetryMessage with vin, state fields, and current timestamp")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-04-10: Telemetry on-change — build_telemetry reflects current state
    #[test]
    fn test_telemetry_on_change() {
        let state = TelemetryState {
            is_locked: Some(true),
            latitude: Some(48.85),
            longitude: Some(2.35),
            parking_active: None,
        };
        let msg = build_telemetry("WDB123", &state);
        assert_eq!(msg.vin, "WDB123");
        assert_eq!(msg.is_locked, Some(true));
        assert_eq!(msg.latitude, Some(48.85));
        assert_eq!(msg.longitude, Some(2.35));
        assert_eq!(msg.parking_active, None);
    }

    // TS-04-11: Telemetry payload format — all fields present
    #[test]
    fn test_telemetry_all_fields() {
        let state = TelemetryState {
            is_locked: Some(true),
            latitude: Some(48.85),
            longitude: Some(2.35),
            parking_active: Some(true),
        };
        let msg = build_telemetry("WDB123", &state);
        let json = serde_json::to_string(&msg).expect("should serialize to JSON");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("should parse JSON");

        assert_eq!(parsed["vin"], "WDB123");
        assert_eq!(parsed["is_locked"], true);
        assert_eq!(parsed["latitude"], 48.85);
        assert_eq!(parsed["longitude"], 2.35);
        assert_eq!(parsed["parking_active"], true);
        assert!(parsed["timestamp"].as_i64().unwrap_or(0) > 0, "timestamp must be positive");
    }

    // TS-04-E7: Unset telemetry signals omitted from payload
    #[test]
    fn test_telemetry_omit_unset() {
        let state = TelemetryState {
            is_locked: Some(true),
            latitude: None,
            longitude: None,
            parking_active: None,
        };
        let msg = build_telemetry("VIN", &state);
        let json = serde_json::to_string(&msg).expect("should serialize to JSON");

        assert!(!json.contains("latitude"), "should omit latitude when None");
        assert!(!json.contains("longitude"), "should omit longitude when None");
        assert!(!json.contains("parking_active"), "should omit parking_active when None");
        assert!(json.contains("is_locked"), "should include is_locked when Some");
        assert!(json.contains("VIN"), "should include vin");
    }
}
