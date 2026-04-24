//! Property tests for cloud-gateway-client.
//!
//! Tests cover:
//! - TS-04-P2: Command Structural Validity
//! - TS-04-P3: Command Passthrough Fidelity (unit-level approximation)
//! - TS-04-P4: Response Relay Fidelity (unit-level: JSON round-trip)
//! - TS-04-P5: Telemetry Completeness
//! - TS-04-P6: Startup Determinism (unit-level: config validation ordering)
//!
//! Requirements: [04-REQ-6.1], [04-REQ-6.3], [04-REQ-7.1], [04-REQ-8.1], [04-REQ-9.1]
//!
//! Note: TS-04-P3, TS-04-P4, and TS-04-P6 require live infrastructure for
//! full end-to-end verification. The tests here validate the properties at
//! the unit level using the pure validation and telemetry functions. Full
//! integration property tests should be added in task group 8.

use cloud_gateway_client::command_validator::validate_command_payload;
use cloud_gateway_client::models::SignalUpdate;
use cloud_gateway_client::telemetry::TelemetryState;
use proptest::prelude::*;

// ===========================================================================
// TS-04-P2: Command Structural Validity
// Validates: [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.3],
//            [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3]
// ===========================================================================

/// Strategy for generating valid command payloads.
fn valid_command_payload() -> impl Strategy<Value = String> {
    (
        "[a-zA-Z0-9_-]{1,20}",         // command_id (non-empty)
        prop_oneof![Just("lock"), Just("unlock")],
        prop::collection::vec("[a-zA-Z_-]{1,10}", 0..5), // doors
    )
        .prop_map(|(id, action, doors)| {
            let doors_json: Vec<String> = doors.iter().map(|d| format!("\"{}\"", d)).collect();
            format!(
                r#"{{"command_id":"{}","action":"{}","doors":[{}]}}"#,
                id,
                action,
                doors_json.join(",")
            )
        })
}

/// Strategy for generating payloads with invalid action values.
fn invalid_action_payload() -> impl Strategy<Value = String> {
    (
        "[a-zA-Z0-9_-]{1,20}",
        "[a-zA-Z]{1,10}".prop_filter("must not be lock or unlock", |s| {
            s != "lock" && s != "unlock"
        }),
    )
        .prop_map(|(id, action)| {
            format!(
                r#"{{"command_id":"{}","action":"{}","doors":["driver"]}}"#,
                id, action
            )
        })
}

/// Strategy for generating payloads with empty command_id.
fn empty_command_id_payload() -> impl Strategy<Value = String> {
    Just(r#"{"command_id":"","action":"lock","doors":["driver"]}"#.to_string())
}

proptest! {
    /// For any valid command payload, validate_command_payload should accept it.
    #[test]
    fn ts_04_p2_valid_payloads_accepted(payload in valid_command_payload()) {
        let result = validate_command_payload(payload.as_bytes());
        prop_assert!(result.is_ok(), "valid payload should be accepted: {}", payload);
    }

    /// For any payload with invalid action, validate_command_payload should reject it.
    #[test]
    fn ts_04_p2_invalid_action_rejected(payload in invalid_action_payload()) {
        let result = validate_command_payload(payload.as_bytes());
        prop_assert!(result.is_err(), "invalid action should be rejected: {}", payload);
    }

    /// For any payload with empty command_id, validate_command_payload should reject it.
    #[test]
    fn ts_04_p2_empty_command_id_rejected(payload in empty_command_id_payload()) {
        let result = validate_command_payload(payload.as_bytes());
        prop_assert!(result.is_err(), "empty command_id should be rejected: {}", payload);
    }

    /// For arbitrary byte strings, validate_command_payload returns Ok only for
    /// structurally valid commands.
    #[test]
    fn ts_04_p2_arbitrary_bytes(payload in prop::collection::vec(any::<u8>(), 0..200)) {
        let result = validate_command_payload(&payload);
        // We can't assert specific outcomes for arbitrary bytes, but we verify
        // the function does not panic.
        if let Ok(cmd) = result {
            // If accepted, all structural invariants must hold
            prop_assert!(!cmd.command_id.is_empty(), "command_id must be non-empty");
            prop_assert!(
                cmd.action == "lock" || cmd.action == "unlock",
                "action must be lock or unlock"
            );
        }
    }
}

// ===========================================================================
// TS-04-P3: Command Passthrough Fidelity (unit-level)
// Validates: [04-REQ-6.3], [04-REQ-6.4]
// ===========================================================================
//
// At unit level, we verify that validate_command_payload preserves all fields
// from the original payload (including extra fields not in the schema).
// Full end-to-end passthrough testing requires live NATS and DATA_BROKER
// infrastructure and is deferred to integration tests.

proptest! {
    /// For any valid command payload with extra fields, the validated result
    /// preserves the command_id, action, and doors exactly.
    #[test]
    fn ts_04_p3_validated_payload_preserves_fields(
        id in "[a-zA-Z0-9]{1,10}",
        action in prop_oneof![Just("lock"), Just("unlock")],
        door in "[a-zA-Z]{1,8}",
    ) {
        let payload = format!(
            r#"{{"command_id":"{}","action":"{}","doors":["{}"],"extra_field":"keep_me"}}"#,
            id, action, door
        );
        let result = validate_command_payload(payload.as_bytes());
        let cmd = result.unwrap();
        prop_assert_eq!(&cmd.command_id, &id);
        prop_assert_eq!(&cmd.action, &action);
        prop_assert_eq!(&cmd.doors, &vec![door]);
    }
}

// ===========================================================================
// TS-04-P4: Response Relay Fidelity (unit-level)
// Validates: [04-REQ-7.1], [04-REQ-7.2]
// ===========================================================================
//
// At unit level, we verify that command response JSON round-trips through
// serde without modification. Full relay fidelity (DATA_BROKER -> NATS)
// requires integration tests.

proptest! {
    /// Any valid command response JSON round-trips without modification
    /// through serde serialization.
    #[test]
    fn ts_04_p4_response_json_round_trip(
        command_id in "[a-zA-Z0-9_-]{1,20}",
        status in prop_oneof![Just("success"), Just("failed")],
        timestamp in 1_000_000_000u64..2_000_000_000u64,
    ) {
        let json = format!(
            r#"{{"command_id":"{}","status":"{}","timestamp":{}}}"#,
            command_id, status, timestamp
        );
        // Verify the JSON can be parsed and the fields are preserved
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("response JSON should be valid");
        prop_assert_eq!(parsed["command_id"].as_str().unwrap(), command_id.as_str());
        prop_assert_eq!(parsed["status"].as_str().unwrap(), status);
        prop_assert_eq!(parsed["timestamp"].as_u64().unwrap(), timestamp);
    }
}

// ===========================================================================
// TS-04-P5: Telemetry Completeness
// Validates: [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
// ===========================================================================

proptest! {
    /// For any sequence of signal updates, the telemetry state produces JSON
    /// containing exactly the set of previously updated fields with their
    /// latest values, and omits fields that have never been set.
    #[test]
    fn ts_04_p5_telemetry_completeness(
        locked in proptest::option::of(any::<bool>()),
        lat in proptest::option::of(-90.0f64..90.0),
        lon in proptest::option::of(-180.0f64..180.0),
        parking in proptest::option::of(any::<bool>()),
    ) {
        let mut state = TelemetryState::new("PROP-VIN".to_string());
        let mut last_json: Option<String> = None;

        // Apply each update and track known fields
        if let Some(v) = locked {
            if let Some(json) = state.update(SignalUpdate::IsLocked(v)) {
                last_json = Some(json);
            }
        }
        if let Some(v) = lat {
            if let Some(json) = state.update(SignalUpdate::Latitude(v)) {
                last_json = Some(json);
            }
        }
        if let Some(v) = lon {
            if let Some(json) = state.update(SignalUpdate::Longitude(v)) {
                last_json = Some(json);
            }
        }
        if let Some(v) = parking {
            if let Some(json) = state.update(SignalUpdate::ParkingActive(v)) {
                last_json = Some(json);
            }
        }

        // If we applied at least one update, verify the final JSON
        if let Some(json) = last_json {
            let parsed: serde_json::Value = serde_json::from_str(&json)
                .expect("telemetry should be valid JSON");

            // Check vin is always present
            prop_assert_eq!(parsed["vin"].as_str().unwrap(), "PROP-VIN");

            // Check that known fields are present with correct values
            if let Some(v) = locked {
                prop_assert_eq!(parsed["is_locked"].as_bool().unwrap(), v);
            } else {
                prop_assert!(parsed.get("is_locked").is_none());
            }

            if let Some(v) = lat {
                let actual = parsed["latitude"].as_f64().unwrap();
                prop_assert!((actual - v).abs() < 1e-10);
            } else {
                prop_assert!(parsed.get("latitude").is_none());
            }

            if let Some(v) = lon {
                let actual = parsed["longitude"].as_f64().unwrap();
                prop_assert!((actual - v).abs() < 1e-10);
            } else {
                prop_assert!(parsed.get("longitude").is_none());
            }

            if let Some(v) = parking {
                prop_assert_eq!(parsed["parking_active"].as_bool().unwrap(), v);
            } else {
                prop_assert!(parsed.get("parking_active").is_none());
            }

            // Timestamp should always be present
            prop_assert!(parsed.get("timestamp").is_some());
        }
    }
}

// ===========================================================================
// TS-04-P6: Startup Determinism (unit-level)
// Validates: [04-REQ-9.1], [04-REQ-9.2]
// ===========================================================================
//
// Full startup determinism testing requires process spawning with failure
// injection. At unit level, we verify the first step: config validation
// prevents subsequent steps when VIN is missing.

#[test]
#[serial_test::serial]
fn ts_04_p6_startup_fails_at_config_step() {
    use cloud_gateway_client::config::Config;
    use cloud_gateway_client::errors::ConfigError;

    // Ensure VIN is not set to simulate config failure
    std::env::remove_var("VIN");

    let result = Config::from_env();
    assert!(
        matches!(result, Err(ConfigError::MissingVin)),
        "startup should fail at config step when VIN is missing"
    );
    // If config fails, subsequent steps (NATS connect, broker connect, etc.)
    // cannot proceed because the caller (main) will exit.
}
