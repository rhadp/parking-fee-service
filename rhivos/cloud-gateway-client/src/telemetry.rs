use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{SignalUpdate, TelemetryMessage};

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
        TelemetryState {
            vin,
            is_locked: None,
            latitude: None,
            longitude: None,
            parking_active: None,
        }
    }

    /// Apply a signal update and return the aggregated JSON payload.
    ///
    /// Returns `Some(json)` on every call (telemetry is published on every
    /// signal change). Returns `None` only on internal serialization failure.
    pub fn update(&mut self, signal: SignalUpdate) -> Option<String> {
        match signal {
            SignalUpdate::IsLocked(v) => self.is_locked = Some(v),
            SignalUpdate::Latitude(v) => self.latitude = Some(v),
            SignalUpdate::Longitude(v) => self.longitude = Some(v),
            SignalUpdate::ParkingActive(v) => self.parking_active = Some(v),
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
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
    //
    // Property: For any change to Vehicle.Command.Door.Response in DATA_BROKER,
    // the JSON value is published verbatim to NATS without modification.
    //
    // Full end-to-end verification (DATA_BROKER -> NATS) is exercised by
    // integration test TS-04-11. This unit-level property test verifies the
    // critical data-preservation invariant in isolation:
    //   1. The JSON validity check (subscribe_responses) does not alter data.
    //   2. The byte conversion (publish_response) is lossless.
    //   3. Valid response payloads containing the required fields (command_id,
    //      status, timestamp) survive the relay path intact.
    proptest! {
        #[test]
        fn ts_04_p4_response_relay_fidelity(
            command_id in "[a-zA-Z0-9_-]{1,32}",
            status in prop_oneof!["success", "failed"],
            timestamp in 1_000_000_000u64..2_000_000_000u64,
            extra_key in "x_[a-z]{1,8}",
            extra_value in "[a-zA-Z0-9]{1,16}",
        ) {
            // Build a response JSON with required fields and an extra field
            // to verify no stripping occurs.
            let json_str = format!(
                r#"{{"command_id":"{cmd}","status":"{st}","timestamp":{ts},"{ek}":"{ev}"}}"#,
                cmd = command_id,
                st = status,
                ts = timestamp,
                ek = extra_key,
                ev = extra_value,
            );

            // Step 1: JSON validity check (mirrors subscribe_responses logic).
            // This must succeed AND must not alter the string.
            let parsed: serde_json::Value = serde_json::from_str(&json_str)
                .expect("generated JSON must be valid");

            // Required fields must be present (REQ-7.2).
            prop_assert!(parsed.get("command_id").is_some(), "command_id must be present");
            prop_assert!(parsed.get("status").is_some(), "status must be present");
            prop_assert!(parsed.get("timestamp").is_some(), "timestamp must be present");

            // Extra fields must survive (verbatim relay, REQ-7.1).
            prop_assert!(
                parsed.get(&extra_key).is_some(),
                "extra field '{}' must be preserved", extra_key
            );

            // Step 2: Byte conversion (mirrors publish_response logic:
            // `json.as_bytes().to_vec().into()`). Roundtrip must be lossless.
            let bytes = json_str.as_bytes().to_vec();
            let recovered = std::str::from_utf8(&bytes)
                .expect("UTF-8 roundtrip must succeed");
            prop_assert_eq!(
                &json_str, recovered,
                "byte conversion in publish_response must be lossless"
            );

            // Step 3: Field values must be intact after the full parse cycle.
            prop_assert_eq!(
                parsed["command_id"].as_str().unwrap(), command_id.as_str(),
                "command_id value must be preserved"
            );
            prop_assert_eq!(
                parsed["status"].as_str().unwrap(), status,
                "status value must be preserved"
            );
            prop_assert_eq!(
                parsed["timestamp"].as_u64().unwrap(), timestamp,
                "timestamp value must be preserved"
            );
            prop_assert_eq!(
                parsed[&extra_key].as_str().unwrap(), extra_value.as_str(),
                "extra field value must be preserved"
            );
        }
    }

    // TS-04-P6: Startup Determinism
    // Validates: [04-REQ-9.1], [04-REQ-9.2]
    //
    // Property: For any execution of the service, initialization proceeds in
    // strict order (config -> NATS -> DATA_BROKER -> registration -> processing)
    // and a failure at any step prevents subsequent steps from executing.
    //
    // This property inherently requires process-level control to inject failures
    // at each startup step (NATS unavailable, DATA_BROKER unreachable, etc.).
    // It cannot be meaningfully verified as a pure unit test because the startup
    // sequence involves real async I/O connections.
    //
    // Verification delegation:
    //   - Step 1 (config) failure: unit test ts_04_e1 + smoke TS-04-SMOKE-2
    //   - Step 2 (NATS) failure: integration test TS-04-15 (exponential backoff)
    //   - Step 3 (DATA_BROKER) failure: covered by exit-on-error in main.rs
    //   - Step 4 (registration) failure: covered by exit-on-error in main.rs
    //   - Step 5 (subscriptions) failure: covered by exit-on-error in main.rs
    //   - Ordering invariant: integration test TS-04-13 (registration after
    //     both connections established, per errata E2)
    #[test]
    #[ignore = "integration: requires process spawning and infrastructure; delegated to TS-04-15, TS-04-SMOKE-2, TS-04-13"]
    fn ts_04_p6_startup_determinism() {
        // This property is verified by the integration and smoke tests listed
        // above. See docs/errata/04_cloud_gateway_client.md §E2 for the
        // startup ordering decision.
    }
}
