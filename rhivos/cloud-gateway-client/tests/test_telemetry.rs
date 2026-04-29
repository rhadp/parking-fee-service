//! Unit tests for the telemetry module.
//!
//! Test Spec: TS-04-7, TS-04-8, TS-04-9
//! Requirements: 04-REQ-8.1, 04-REQ-8.2, 04-REQ-8.3

use cloud_gateway_client::models::SignalUpdate;
use cloud_gateway_client::telemetry::TelemetryState;

/// TS-04-7: Telemetry state produces JSON on first update.
///
/// Requirements: 04-REQ-8.1, 04-REQ-8.2
/// WHEN the first signal update is applied, the system SHALL produce a
/// telemetry JSON with the updated field and omit unset fields.
#[test]
fn test_telemetry_first_update_produces_json() {
    let mut state = TelemetryState::new("VIN-001".to_string());

    let result = state.update(SignalUpdate::IsLocked(true));

    assert!(result.is_some(), "First update should produce JSON");
    let json = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");

    assert_eq!(parsed["vin"], "VIN-001");
    assert_eq!(parsed["is_locked"], true);
    assert!(parsed.get("timestamp").is_some(), "Should have timestamp");
    assert!(parsed.get("latitude").is_none(), "Latitude should be omitted");
    assert!(
        parsed.get("longitude").is_none(),
        "Longitude should be omitted"
    );
    assert!(
        parsed.get("parking_active").is_none(),
        "ParkingActive should be omitted"
    );
}

/// TS-04-8: Telemetry state omits unset fields.
///
/// Requirement: 04-REQ-8.3
/// WHEN a signal has never been set, the corresponding field SHALL be
/// omitted from the telemetry payload.
#[test]
fn test_telemetry_omits_unset_fields() {
    let mut state = TelemetryState::new("VIN-001".to_string());

    let result = state.update(SignalUpdate::Latitude(48.1351));

    assert!(result.is_some(), "Update should produce JSON");
    let json = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");

    assert_eq!(parsed["latitude"], 48.1351);
    assert!(
        parsed.get("is_locked").is_none(),
        "is_locked should be omitted"
    );
    assert!(
        parsed.get("longitude").is_none(),
        "longitude should be omitted"
    );
    assert!(
        parsed.get("parking_active").is_none(),
        "parking_active should be omitted"
    );
}

/// TS-04-9: Telemetry state includes all known fields after multiple updates.
///
/// Requirement: 04-REQ-8.2
/// WHEN multiple signals have been updated, all known fields SHALL appear
/// in the telemetry payload.
#[test]
fn test_telemetry_includes_all_known_fields() {
    let mut state = TelemetryState::new("VIN-001".to_string());

    // Apply updates for all signal types
    let _ = state.update(SignalUpdate::IsLocked(true));
    let _ = state.update(SignalUpdate::Latitude(48.1351));
    let _ = state.update(SignalUpdate::Longitude(11.582));
    let result = state.update(SignalUpdate::ParkingActive(true));

    assert!(
        result.is_some(),
        "Update should produce JSON after all signals set"
    );
    let json = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");

    assert_eq!(parsed["is_locked"], true);
    assert_eq!(parsed["latitude"], 48.1351);
    assert_eq!(parsed["longitude"], 11.582);
    assert_eq!(parsed["parking_active"], true);
    assert_eq!(parsed["vin"], "VIN-001");
    assert!(parsed.get("timestamp").is_some(), "Should have timestamp");
}
