use crate::models::SignalUpdate;

/// Accumulates the latest value for each VSS telemetry signal and produces
/// an aggregated JSON payload whenever any signal changes.
///
/// Fields that have never been updated are omitted from the payload (REQ-8.3).
pub struct TelemetryState {
    vin: String,
    is_locked: Option<bool>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    parking_active: Option<bool>,
}

impl TelemetryState {
    /// Create a new, empty telemetry state for the given VIN.
    pub fn new(vin: String) -> Self {
        todo!("implement TelemetryState::new")
    }

    /// Apply a signal update and return the aggregated JSON payload.
    ///
    /// Returns `Some(json)` on every call (telemetry is published on every
    /// signal change). Returns `None` only on internal serialization failure.
    pub fn update(&mut self, signal: SignalUpdate) -> Option<String> {
        todo!("implement TelemetryState::update")
    }
}

// ─────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // TS-04-7: Telemetry state produces JSON on first update, omitting unset fields
    // Validates: [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
    #[test]
    fn ts_04_7_telemetry_first_update_is_locked() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        let result = state.update(SignalUpdate::IsLocked(true));

        let json_str = result.expect("should return Some(json)");
        let json: serde_json::Value =
            serde_json::from_str(&json_str).expect("must be valid JSON");

        assert_eq!(json["vin"], "VIN-001");
        assert_eq!(json["is_locked"], true);
        assert!(json["timestamp"].is_number(), "timestamp must be present");
        assert!(json.get("latitude").is_none(), "latitude must be absent");
        assert!(json.get("longitude").is_none(), "longitude must be absent");
        assert!(json.get("parking_active").is_none(), "parking_active must be absent");
    }

    // TS-04-8: Telemetry state omits unset fields
    // Validates: [04-REQ-8.3]
    #[test]
    fn ts_04_8_telemetry_omits_unset_fields() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        let result = state.update(SignalUpdate::Latitude(48.1351));

        let json_str = result.expect("should return Some(json)");
        let json: serde_json::Value = serde_json::from_str(&json_str).expect("valid JSON");

        assert!(
            (json["latitude"].as_f64().unwrap() - 48.1351).abs() < 1e-6,
            "latitude must be 48.1351"
        );
        assert!(json.get("is_locked").is_none(), "is_locked must be absent");
        assert!(json.get("longitude").is_none(), "longitude must be absent");
        assert!(json.get("parking_active").is_none(), "parking_active must be absent");
    }

    // TS-04-9: Telemetry state includes all known fields after multiple updates
    // Validates: [04-REQ-8.2]
    #[test]
    fn ts_04_9_telemetry_includes_all_known_fields() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        state.update(SignalUpdate::IsLocked(true));
        state.update(SignalUpdate::Latitude(48.1351));
        state.update(SignalUpdate::Longitude(11.582));
        let result = state.update(SignalUpdate::ParkingActive(true));

        let json_str = result.expect("should return Some(json)");
        let json: serde_json::Value = serde_json::from_str(&json_str).expect("valid JSON");

        assert_eq!(json["is_locked"], true);
        assert!(
            (json["latitude"].as_f64().unwrap() - 48.1351).abs() < 1e-6,
            "latitude must be 48.1351"
        );
        assert!(
            (json["longitude"].as_f64().unwrap() - 11.582).abs() < 1e-6,
            "longitude must be 11.582"
        );
        assert_eq!(json["parking_active"], true);
    }

    // TS-04-P5: Telemetry Completeness
    // Validates: [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
    //
    // For any sequence of signal updates, the published JSON contains exactly the
    // set of previously updated fields, each with its latest value.
    proptest! {
        #[test]
        fn ts_04_p5_telemetry_completeness(
            is_locked_val in proptest::option::of(proptest::bool::ANY),
            latitude_val in proptest::option::of(-90.0f64..=90.0),
            longitude_val in proptest::option::of(-180.0f64..=180.0),
            parking_val in proptest::option::of(proptest::bool::ANY),
        ) {
            let mut state = TelemetryState::new("VIN".to_string());
            let mut last_json: Option<serde_json::Value> = None;

            // Apply updates in order; track what is "known".
            if let Some(v) = is_locked_val {
                if let Some(j) = state.update(SignalUpdate::IsLocked(v)) {
                    last_json = Some(serde_json::from_str(&j).unwrap());
                }
            }
            if let Some(v) = latitude_val {
                if let Some(j) = state.update(SignalUpdate::Latitude(v)) {
                    last_json = Some(serde_json::from_str(&j).unwrap());
                }
            }
            if let Some(v) = longitude_val {
                if let Some(j) = state.update(SignalUpdate::Longitude(v)) {
                    last_json = Some(serde_json::from_str(&j).unwrap());
                }
            }
            if let Some(v) = parking_val {
                if let Some(j) = state.update(SignalUpdate::ParkingActive(v)) {
                    last_json = Some(serde_json::from_str(&j).unwrap());
                }
            }

            // If at least one update was applied, verify the JSON.
            if let Some(json) = last_json {
                // Fields that were set must be present.
                if is_locked_val.is_some() {
                    prop_assert!(json.get("is_locked").is_some(), "is_locked must be present");
                }
                if latitude_val.is_some() {
                    prop_assert!(json.get("latitude").is_some(), "latitude must be present");
                }
                if longitude_val.is_some() {
                    prop_assert!(json.get("longitude").is_some(), "longitude must be present");
                }
                if parking_val.is_some() {
                    prop_assert!(json.get("parking_active").is_some(), "parking_active must be present");
                }
                // Fields that were never set must be absent.
                if is_locked_val.is_none() {
                    prop_assert!(json.get("is_locked").is_none(), "is_locked must be absent");
                }
                if latitude_val.is_none() {
                    prop_assert!(json.get("latitude").is_none(), "latitude must be absent");
                }
                if longitude_val.is_none() {
                    prop_assert!(json.get("longitude").is_none(), "longitude must be absent");
                }
                if parking_val.is_none() {
                    prop_assert!(json.get("parking_active").is_none(), "parking_active must be absent");
                }
            }
        }
    }

    // TS-04-P4: Response Relay Fidelity
    // Validates: [04-REQ-7.1], [04-REQ-7.2]
    // This property requires a running DATA_BROKER and NATS server; it is
    // exercised by integration test TS-04-11 in task group 8.
    #[test]
    #[ignore = "integration: requires NATS + DATA_BROKER containers"]
    fn ts_04_p4_response_relay_fidelity() {
        // Verified by integration test TS-04-11.
    }

    // TS-04-P6: Startup Determinism
    // Validates: [04-REQ-9.1], [04-REQ-9.2]
    // This property requires process-level control; it is exercised by
    // integration/smoke tests in task group 8/9.
    #[test]
    #[ignore = "integration: requires process spawning and infrastructure"]
    fn ts_04_p6_startup_determinism() {
        // Verified by integration smoke tests.
    }
}
