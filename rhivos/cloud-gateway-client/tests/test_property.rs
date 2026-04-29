//! Property tests for CLOUD_GATEWAY_CLIENT correctness properties.
//!
//! Test Spec: TS-04-P1, TS-04-P2, TS-04-P3, TS-04-P4, TS-04-P5, TS-04-P6
//! Requirements: 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.E1, 04-REQ-5.E2,
//!               04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.E1, 04-REQ-6.E2, 04-REQ-6.E3,
//!               04-REQ-6.3, 04-REQ-6.4,
//!               04-REQ-7.1, 04-REQ-7.2,
//!               04-REQ-8.1, 04-REQ-8.2, 04-REQ-8.3,
//!               04-REQ-9.1, 04-REQ-9.2

use std::collections::HashMap;

use proptest::prelude::*;

use cloud_gateway_client::command_validator::{validate_bearer_token, validate_command_payload};
use cloud_gateway_client::models::SignalUpdate;
use cloud_gateway_client::telemetry::TelemetryState;

// ---------------------------------------------------------------------------
// TS-04-P1: Command Authentication Integrity (Property 1)
//
// For any NATS message, the system accepts it only if the Authorization
// header matches the configured bearer token.
//
// Validates: 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.E1, 04-REQ-5.E2
// ---------------------------------------------------------------------------

proptest! {
    /// For any header value and expected token, validate_bearer_token returns
    /// Ok if and only if the header is exactly "Bearer <expected_token>".
    #[test]
    fn prop_auth_integrity_with_header(
        header_value in ".{0,100}",
        expected_token in "[a-zA-Z0-9_-]{1,20}",
    ) {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), header_value.clone());

        let result = validate_bearer_token(&headers, &expected_token);
        let should_be_valid = header_value == format!("Bearer {}", expected_token);

        if should_be_valid {
            prop_assert!(result.is_ok(), "Expected Ok for matching token");
        } else {
            prop_assert!(result.is_err(), "Expected Err for non-matching header value");
        }
    }

    /// For any expected token, a missing Authorization header always fails.
    #[test]
    fn prop_auth_missing_header_always_fails(
        expected_token in "[a-zA-Z0-9_-]{1,20}",
    ) {
        let headers = HashMap::new();

        let result = validate_bearer_token(&headers, &expected_token);
        prop_assert!(result.is_err(), "Missing header should always fail");
    }
}

// ---------------------------------------------------------------------------
// TS-04-P2: Command Structural Validity (Property 2)
//
// For any command that passes authentication, the system writes to
// DATA_BROKER only if the payload is valid JSON containing a non-empty
// command_id, a valid action, and a doors array.
//
// Validates: 04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.E1, 04-REQ-6.E2, 04-REQ-6.E3
// ---------------------------------------------------------------------------

proptest! {
    /// For any combination of present/missing fields and action values,
    /// validate_command_payload returns Ok iff all required fields are
    /// present with valid values.
    #[test]
    fn prop_command_structural_validity(
        has_command_id in any::<bool>(),
        command_id in "[a-zA-Z0-9_-]{0,20}",
        has_action in any::<bool>(),
        action in "[a-zA-Z]{0,10}",
        has_doors in any::<bool>(),
    ) {
        let mut map = serde_json::Map::new();
        if has_command_id {
            map.insert("command_id".into(), serde_json::json!(command_id));
        }
        if has_action {
            map.insert("action".into(), serde_json::json!(action));
        }
        if has_doors {
            map.insert("doors".into(), serde_json::json!(["driver"]));
        }

        let payload = serde_json::to_vec(&map).unwrap();
        let result = validate_command_payload(&payload);

        let should_pass = has_command_id
            && !command_id.is_empty()
            && has_action
            && (action == "lock" || action == "unlock")
            && has_doors;

        if should_pass {
            prop_assert!(result.is_ok(), "Expected Ok for valid payload, got {:?}", result);
        } else {
            prop_assert!(result.is_err(), "Expected Err for invalid payload");
        }
    }

    /// Invalid JSON always fails validation.
    #[test]
    fn prop_command_invalid_bytes_fail(
        garbage in proptest::collection::vec(any::<u8>(), 1..50),
    ) {
        // Only test if the bytes are NOT accidentally valid JSON with required fields
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&garbage) {
            if val.is_object() {
                // Skip: might accidentally be valid JSON
                return Ok(());
            }
        }
        let result = validate_command_payload(&garbage);
        prop_assert!(result.is_err(), "Non-JSON bytes should fail validation");
    }
}

// ---------------------------------------------------------------------------
// TS-04-P3: Command Passthrough Fidelity (Property 3)
//
// For any validated command, the payload written to DATA_BROKER is identical
// to the original payload received from NATS.
//
// Validates: 04-REQ-6.3, 04-REQ-6.4
// ---------------------------------------------------------------------------

/// Property test for command passthrough fidelity.
///
/// NOTE: This is an integration-level property test that requires
/// NATS and DATA_BROKER infrastructure. Deferred to task group 8.
#[test]
#[ignore]
fn prop_command_passthrough_fidelity() {
    // For any valid command payload, verify that the bytes written to
    // Vehicle.Command.Door.Lock in DATA_BROKER are identical to the
    // original NATS message payload.
    todo!("Integration property test: requires NATS and DATA_BROKER")
}

// ---------------------------------------------------------------------------
// TS-04-P4: Response Relay Fidelity (Property 4)
//
// For any change to Vehicle.Command.Door.Response in DATA_BROKER, the JSON
// value is published verbatim to NATS without modification.
//
// Validates: 04-REQ-7.1, 04-REQ-7.2
// ---------------------------------------------------------------------------

/// Property test for response relay fidelity.
///
/// NOTE: This is an integration-level property test that requires
/// NATS and DATA_BROKER infrastructure. Deferred to task group 8.
#[test]
#[ignore]
fn prop_response_relay_fidelity() {
    // For any JSON string written to Vehicle.Command.Door.Response,
    // verify the NATS message on vehicles.{VIN}.command_responses
    // contains the identical bytes.
    todo!("Integration property test: requires NATS and DATA_BROKER")
}

// ---------------------------------------------------------------------------
// TS-04-P5: Telemetry Completeness (Property 5)
//
// For any change to a subscribed telemetry signal, the system publishes
// an aggregated JSON message that includes all currently known signal
// values and omits signals never set.
//
// Validates: 04-REQ-8.1, 04-REQ-8.2, 04-REQ-8.3
// ---------------------------------------------------------------------------

/// Generate an arbitrary SignalUpdate.
fn arb_signal_update() -> impl Strategy<Value = SignalUpdate> {
    prop_oneof![
        any::<bool>().prop_map(SignalUpdate::IsLocked),
        (-90.0f64..90.0).prop_map(SignalUpdate::Latitude),
        (-180.0f64..180.0).prop_map(SignalUpdate::Longitude),
        any::<bool>().prop_map(SignalUpdate::ParkingActive),
    ]
}

proptest! {
    /// For any sequence of signal updates, the telemetry JSON contains
    /// exactly the set of previously updated fields with their latest values.
    #[test]
    fn prop_telemetry_completeness(
        updates in proptest::collection::vec(arb_signal_update(), 1..10),
    ) {
        let mut state = TelemetryState::new("VIN".to_string());
        let mut known_is_locked: Option<bool> = None;
        let mut known_latitude: Option<f64> = None;
        let mut known_longitude: Option<f64> = None;
        let mut known_parking: Option<bool> = None;

        for update in updates {
            match &update {
                SignalUpdate::IsLocked(v) => known_is_locked = Some(*v),
                SignalUpdate::Latitude(v) => known_latitude = Some(*v),
                SignalUpdate::Longitude(v) => known_longitude = Some(*v),
                SignalUpdate::ParkingActive(v) => known_parking = Some(*v),
            }

            if let Some(json) = state.update(update) {
                let parsed: serde_json::Value =
                    serde_json::from_str(&json).expect("Should be valid JSON");

                // Check VIN always present
                prop_assert_eq!(parsed["vin"].as_str(), Some("VIN"));

                // Check known fields are present with correct values
                if let Some(v) = known_is_locked {
                    prop_assert_eq!(parsed.get("is_locked").and_then(|x| x.as_bool()), Some(v));
                } else {
                    prop_assert!(parsed.get("is_locked").is_none());
                }

                if let Some(v) = known_latitude {
                    prop_assert_eq!(parsed.get("latitude").and_then(|x| x.as_f64()), Some(v));
                } else {
                    prop_assert!(parsed.get("latitude").is_none());
                }

                if let Some(v) = known_longitude {
                    prop_assert_eq!(parsed.get("longitude").and_then(|x| x.as_f64()), Some(v));
                } else {
                    prop_assert!(parsed.get("longitude").is_none());
                }

                if let Some(v) = known_parking {
                    prop_assert_eq!(
                        parsed.get("parking_active").and_then(|x| x.as_bool()),
                        Some(v)
                    );
                } else {
                    prop_assert!(parsed.get("parking_active").is_none());
                }

                // Timestamp must always be present
                prop_assert!(parsed.get("timestamp").is_some());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TS-04-P6: Startup Determinism (Property 6)
//
// For any execution of the service, initialization proceeds in strict order
// and a failure at any step prevents subsequent steps from executing.
//
// Validates: 04-REQ-9.1, 04-REQ-9.2
// ---------------------------------------------------------------------------

/// Property test for startup determinism.
///
/// NOTE: This is an integration-level property test that requires
/// injecting failures at each startup step. Deferred to task group 8.
#[test]
#[ignore]
fn prop_startup_determinism() {
    // For any failure step in [config, nats_connect, broker_connect,
    // registration, processing], verify:
    // 1. Steps before the failure step all completed
    // 2. Steps after the failure step none executed
    // 3. Service exits with non-zero code
    todo!("Integration property test: requires failure injection")
}
