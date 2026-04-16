use crate::models::{SignalUpdate, TelemetryMessage};

/// Maintains the current aggregated telemetry state for a vehicle.
///
/// Fields are `Option<T>` — `None` means "never received a value from
/// DATA_BROKER for this signal". On serialization those fields are omitted
/// (per REQ-8.3).
pub struct TelemetryState {
    vin: String,
    is_locked: Option<bool>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    parking_active: Option<bool>,
}

impl TelemetryState {
    /// Create a new, empty telemetry state for the given VIN.
    pub fn new(vin: impl Into<String>) -> Self {
        TelemetryState {
            vin: vin.into(),
            is_locked: None,
            latitude: None,
            longitude: None,
            parking_active: None,
        }
    }

    /// Apply a signal update and return the serialized JSON telemetry payload.
    /// Always returns `Some(json)` after any update (REQ-8.1).
    pub fn update(&mut self, signal: SignalUpdate) -> Option<String> {
        match signal {
            SignalUpdate::IsLocked(v) => self.is_locked = Some(v),
            SignalUpdate::Latitude(v) => self.latitude = Some(v),
            SignalUpdate::Longitude(v) => self.longitude = Some(v),
            SignalUpdate::ParkingActive(v) => self.parking_active = Some(v),
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let msg = TelemetryMessage {
            vin: self.vin.clone(),
            is_locked: self.is_locked,
            latitude: self.latitude,
            longitude: self.longitude,
            parking_active: self.parking_active,
            timestamp,
        };

        serde_json::to_string(&msg).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-04-7: Telemetry state produces JSON on first update, omitting unset fields
    #[test]
    fn ts_04_7_telemetry_first_update_is_locked() {
        let mut state = TelemetryState::new("VIN-001");
        let result = state.update(SignalUpdate::IsLocked(true));
        let json = result.expect("expected Some(json) on first update");

        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("telemetry payload must be valid JSON");

        assert_eq!(parsed["vin"], "VIN-001", "vin field must match");
        assert_eq!(parsed["is_locked"], true, "is_locked must be true");
        assert!(parsed.get("timestamp").is_some(), "timestamp must be present");
        assert!(parsed.get("latitude").is_none(), "latitude must be absent (never set)");
        assert!(parsed.get("longitude").is_none(), "longitude must be absent (never set)");
        assert!(
            parsed.get("parking_active").is_none(),
            "parking_active must be absent (never set)"
        );
    }

    // TS-04-8: Telemetry state omits unset fields when only latitude is set
    #[test]
    fn ts_04_8_telemetry_omits_unset_fields() {
        let mut state = TelemetryState::new("VIN-001");
        let result = state.update(SignalUpdate::Latitude(48.1351));
        let json = result.expect("expected Some(json) on first latitude update");

        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("telemetry payload must be valid JSON");

        assert!(
            (parsed["latitude"].as_f64().unwrap() - 48.1351).abs() < 1e-9,
            "latitude must be 48.1351"
        );
        assert!(parsed.get("is_locked").is_none(), "is_locked must be absent");
        assert!(parsed.get("longitude").is_none(), "longitude must be absent");
        assert!(parsed.get("parking_active").is_none(), "parking_active must be absent");
    }

    // TS-04-9: Telemetry state includes all known fields after multiple updates
    #[test]
    fn ts_04_9_telemetry_includes_all_known_fields() {
        let mut state = TelemetryState::new("VIN-001");
        state.update(SignalUpdate::IsLocked(true));
        state.update(SignalUpdate::Latitude(48.1351));
        state.update(SignalUpdate::Longitude(11.582));
        let result = state.update(SignalUpdate::ParkingActive(true));
        let json = result.expect("expected Some(json) after all signal updates");

        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("telemetry payload must be valid JSON");

        assert_eq!(parsed["is_locked"], true, "is_locked must be true");
        assert!(
            (parsed["latitude"].as_f64().unwrap() - 48.1351).abs() < 1e-9,
            "latitude must be 48.1351"
        );
        assert!(
            (parsed["longitude"].as_f64().unwrap() - 11.582).abs() < 1e-9,
            "longitude must be 11.582"
        );
        assert_eq!(parsed["parking_active"], true, "parking_active must be true");
    }

    // TS-04-P5: Telemetry Completeness property
    // After a sequence of signal updates, the telemetry JSON contains exactly
    // the signals that have been set, each with their most recent value.
    #[test]
    fn ts_04_p5_telemetry_completeness_sequence() {
        let mut state = TelemetryState::new("VIN-PROP");
        let mut known: std::collections::HashMap<&str, serde_json::Value> =
            std::collections::HashMap::new();

        let updates = vec![
            SignalUpdate::IsLocked(false),
            SignalUpdate::Latitude(52.5),
            SignalUpdate::IsLocked(true),  // overwrite previous value
        ];

        for update in updates {
            let field = match &update {
                SignalUpdate::IsLocked(v) => { known.insert("is_locked", serde_json::json!(v)); "is_locked" }
                SignalUpdate::Latitude(v) => { known.insert("latitude", serde_json::json!(v)); "latitude" }
                SignalUpdate::Longitude(v) => { known.insert("longitude", serde_json::json!(v)); "longitude" }
                SignalUpdate::ParkingActive(v) => { known.insert("parking_active", serde_json::json!(v)); "parking_active" }
            };
            let result = state.update(update);
            let json = result.expect("expected Some(json) on each update");
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            // Every known field must be present with its latest value
            for (key, expected) in &known {
                assert_eq!(
                    parsed.get(*key).unwrap_or(&serde_json::Value::Null),
                    expected,
                    "field '{}' must have value {:?} in telemetry, got: {}",
                    key, expected, json
                );
            }

            // Fields NOT yet seen must be absent
            let all_fields = ["is_locked", "latitude", "longitude", "parking_active"];
            for f in all_fields.iter() {
                if !known.contains_key(f) {
                    assert!(
                        parsed.get(*f).is_none(),
                        "field '{}' must be absent (never set), but found in: {}",
                        f, json
                    );
                }
            }
            let _ = field; // suppress unused warning
        }
    }

    // TS-04-P4: Response Relay Fidelity — placeholder (requires infrastructure)
    // The broker_client and nats_client modules are wired in task groups 5/6.
    // This test documents the contract; full verification is in integration tests.
    #[test]
    #[ignore = "requires NATS and DATA_BROKER containers (integration test)"]
    fn ts_04_p4_response_relay_fidelity_integration() {
        // Verified in integration tests TS-04-11 (task group 8).
        todo!()
    }

    // TS-04-P6: Startup Determinism — placeholder (requires process-level harness)
    // Verified in integration tests TS-04-SMOKE-1/2 and TS-04-15 (task groups 8/9).
    #[test]
    #[ignore = "requires process-level harness (integration test)"]
    fn ts_04_p6_startup_determinism_integration() {
        // Verified in integration tests TS-04-13 and TS-04-15 (task groups 8/9).
        todo!()
    }
}
