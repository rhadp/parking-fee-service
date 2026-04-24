//! Unit tests for the `telemetry` module.
//!
//! Tests cover:
//! - TS-04-7: Telemetry state produces JSON on first update
//! - TS-04-8: Telemetry state omits unset fields
//! - TS-04-9: Telemetry state includes all known fields
//!
//! Requirements: [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]

use cloud_gateway_client::models::SignalUpdate;
use cloud_gateway_client::telemetry::TelemetryState;

// ---------------------------------------------------------------------------
// TS-04-7: Telemetry state produces JSON on first update
// Validates: [04-REQ-8.1], [04-REQ-8.2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_7_telemetry_first_update() {
    // GIVEN state = TelemetryState::new("VIN-001")
    let mut state = TelemetryState::new("VIN-001".to_string());

    // WHEN state.update(SignalUpdate::IsLocked(true)) is called
    let result = state.update(SignalUpdate::IsLocked(true));

    // THEN result is Some(json)
    let json = result.expect("first update should produce a telemetry message");
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("telemetry should be valid JSON");

    // AND json contains "vin":"VIN-001"
    assert_eq!(parsed["vin"], "VIN-001");
    // AND json contains "is_locked":true
    assert_eq!(parsed["is_locked"], true);
    // AND json contains "timestamp"
    assert!(
        parsed.get("timestamp").is_some(),
        "telemetry should contain a timestamp"
    );
    // AND json does not contain "latitude"
    assert!(
        parsed.get("latitude").is_none(),
        "telemetry should not contain unset latitude"
    );
    // AND json does not contain "longitude"
    assert!(
        parsed.get("longitude").is_none(),
        "telemetry should not contain unset longitude"
    );
    // AND json does not contain "parking_active"
    assert!(
        parsed.get("parking_active").is_none(),
        "telemetry should not contain unset parking_active"
    );
}

// ---------------------------------------------------------------------------
// TS-04-8: Telemetry state omits unset fields
// Validates: [04-REQ-8.3]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_8_telemetry_omits_unset_fields() {
    // GIVEN state = TelemetryState::new("VIN-001")
    let mut state = TelemetryState::new("VIN-001".to_string());

    // WHEN state.update(SignalUpdate::Latitude(48.1351)) is called
    let result = state.update(SignalUpdate::Latitude(48.1351));

    // THEN result is Some(json)
    let json = result.expect("update should produce a telemetry message");
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("telemetry should be valid JSON");

    // AND json contains "latitude":48.1351
    assert_eq!(parsed["latitude"], 48.1351);
    // AND json does not contain "is_locked"
    assert!(
        parsed.get("is_locked").is_none(),
        "telemetry should not contain unset is_locked"
    );
    // AND json does not contain "longitude"
    assert!(
        parsed.get("longitude").is_none(),
        "telemetry should not contain unset longitude"
    );
    // AND json does not contain "parking_active"
    assert!(
        parsed.get("parking_active").is_none(),
        "telemetry should not contain unset parking_active"
    );
}

// ---------------------------------------------------------------------------
// TS-04-9: Telemetry state includes all known fields
// Validates: [04-REQ-8.2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_9_telemetry_includes_all_known_fields() {
    // GIVEN state = TelemetryState::new("VIN-001")
    let mut state = TelemetryState::new("VIN-001".to_string());

    // GIVEN state.update(SignalUpdate::IsLocked(true)) was called
    let _ = state.update(SignalUpdate::IsLocked(true));
    // GIVEN state.update(SignalUpdate::Latitude(48.1351)) was called
    let _ = state.update(SignalUpdate::Latitude(48.1351));
    // GIVEN state.update(SignalUpdate::Longitude(11.582)) was called
    let _ = state.update(SignalUpdate::Longitude(11.582));

    // WHEN state.update(SignalUpdate::ParkingActive(true)) is called
    let result = state.update(SignalUpdate::ParkingActive(true));

    // THEN result is Some(json)
    let json = result.expect("update should produce a telemetry message");
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("telemetry should be valid JSON");

    // AND json contains "is_locked":true
    assert_eq!(parsed["is_locked"], true);
    // AND json contains "latitude":48.1351
    assert_eq!(parsed["latitude"], 48.1351);
    // AND json contains "longitude":11.582
    assert_eq!(parsed["longitude"], 11.582);
    // AND json contains "parking_active":true
    assert_eq!(parsed["parking_active"], true);
}
