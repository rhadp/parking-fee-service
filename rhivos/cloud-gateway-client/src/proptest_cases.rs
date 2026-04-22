//! Property-based tests for cloud-gateway-client.
//!
//! These tests verify invariants across randomized inputs:
//!
//! - TS-04-P2: Command Structural Validity
//! - TS-04-P3: Command Passthrough Fidelity (unit-level validation;
//!   end-to-end verified by TS-04-10 in tests/integration_tests.rs)
//! - TS-04-P4: Response Relay Fidelity (unit-level validation;
//!   end-to-end verified by TS-04-11 in tests/integration_tests.rs)
//! - TS-04-P5: Telemetry Completeness
//! - TS-04-P6: Startup Determinism (config failure at unit level;
//!   NATS/broker failures verified by integration smoke tests)

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::command_validator::validate_command_payload;
    use crate::models::SignalUpdate;
    use crate::telemetry::TelemetryState;

    /// Generate arbitrary command input: a mix of random strings and
    /// structured JSON with variable field presence.
    fn arbitrary_command_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Arbitrary strings (most will be invalid JSON)
            ".{0,100}",
            // Structured JSON objects with random field combinations
            (
                proptest::option::of("[a-z0-9-]{1,20}"),
                proptest::option::of(prop_oneof![
                    Just("lock".to_string()),
                    Just("unlock".to_string()),
                    Just("open".to_string()),
                    "[a-z]{1,10}".prop_map(|s| s),
                ]),
                proptest::option::of(prop::collection::vec("[a-z]{1,10}", 0..5)),
            )
                .prop_map(|(cmd_id, action, doors)| {
                    let mut obj = serde_json::Map::new();
                    if let Some(id) = cmd_id {
                        obj.insert(
                            "command_id".to_string(),
                            serde_json::Value::String(id),
                        );
                    }
                    if let Some(act) = action {
                        obj.insert(
                            "action".to_string(),
                            serde_json::Value::String(act),
                        );
                    }
                    if let Some(d) = doors {
                        let arr: Vec<serde_json::Value> =
                            d.into_iter().map(serde_json::Value::String).collect();
                        obj.insert(
                            "doors".to_string(),
                            serde_json::Value::Array(arr),
                        );
                    }
                    serde_json::to_string(&obj).unwrap()
                }),
        ]
    }

    /// Generate an arbitrary signal update with realistic values.
    fn arbitrary_signal_update() -> impl Strategy<Value = SignalUpdate> {
        prop_oneof![
            any::<bool>().prop_map(SignalUpdate::IsLocked),
            (-90.0f64..90.0f64).prop_map(SignalUpdate::Latitude),
            (-180.0f64..180.0f64).prop_map(SignalUpdate::Longitude),
            any::<bool>().prop_map(SignalUpdate::ParkingActive),
        ]
    }

    // TS-04-P2: Command Structural Validity
    //
    // For any input, validate_command_payload returns Ok if and only if the
    // input is valid JSON containing a non-empty command_id, an action of
    // "lock" or "unlock", and a doors array.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn proptest_command_structural_validity(input in arbitrary_command_input()) {
            let result = validate_command_payload(input.as_bytes());

            // Determine expected outcome by inspecting the input
            let expected_ok = match serde_json::from_str::<serde_json::Value>(&input) {
                Err(_) => false,
                Ok(val) => {
                    let has_command_id = val
                        .get("command_id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.is_empty());
                    let has_valid_action = val
                        .get("action")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| s == "lock" || s == "unlock");
                    let has_doors = val
                        .get("doors")
                        .is_some_and(|v| v.is_array());
                    has_command_id && has_valid_action && has_doors
                }
            };

            if expected_ok {
                prop_assert!(result.is_ok(), "expected Ok for valid payload: {}", input);
            } else {
                prop_assert!(result.is_err(), "expected Err for invalid payload: {}", input);
            }
        }
    }

    // TS-04-P5: Telemetry Completeness
    //
    // For any sequence of signal updates, the published JSON contains
    // exactly the set of previously updated fields, each with its latest
    // value. Fields never set are absent.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn proptest_telemetry_completeness(
            updates in prop::collection::vec(arbitrary_signal_update(), 1..10)
        ) {
            let mut state = TelemetryState::new("VIN-PROP".to_string());
            let mut known_locked: Option<bool> = None;
            let mut known_lat: Option<f64> = None;
            let mut known_lon: Option<f64> = None;
            let mut known_parking: Option<bool> = None;

            for update in updates {
                match &update {
                    SignalUpdate::IsLocked(v) => known_locked = Some(*v),
                    SignalUpdate::Latitude(v) => known_lat = Some(*v),
                    SignalUpdate::Longitude(v) => known_lon = Some(*v),
                    SignalUpdate::ParkingActive(v) => known_parking = Some(*v),
                }

                if let Some(json) = state.update(update) {
                    let parsed: serde_json::Value = serde_json::from_str(&json)
                        .expect("telemetry should be valid JSON");

                    // VIN always present
                    prop_assert_eq!(
                        parsed["vin"].as_str().unwrap(),
                        "VIN-PROP",
                        "vin should match"
                    );

                    // Verify known fields
                    match known_locked {
                        Some(v) => prop_assert_eq!(parsed["is_locked"].as_bool().unwrap(), v),
                        None => prop_assert!(parsed.get("is_locked").is_none()),
                    }
                    match known_lat {
                        Some(v) => prop_assert_eq!(parsed["latitude"].as_f64().unwrap(), v),
                        None => prop_assert!(parsed.get("latitude").is_none()),
                    }
                    match known_lon {
                        Some(v) => prop_assert_eq!(parsed["longitude"].as_f64().unwrap(), v),
                        None => prop_assert!(parsed.get("longitude").is_none()),
                    }
                    match known_parking {
                        Some(v) => prop_assert_eq!(parsed["parking_active"].as_bool().unwrap(), v),
                        None => prop_assert!(parsed.get("parking_active").is_none()),
                    }

                    // Timestamp present
                    prop_assert!(
                        parsed.get("timestamp").is_some(),
                        "timestamp should be present"
                    );
                }
            }
        }
    }

    // TS-04-P3: Command Passthrough Fidelity
    //
    // For any validated command, the payload written to DATA_BROKER is
    // identical to the original NATS payload. At the unit level, we verify
    // that validate_command_payload does not consume or modify the input
    // bytes — the original payload string is preserved and can be forwarded
    // as-is. The end-to-end property is tested by TS-04-10 in
    // tests/integration_tests.rs.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn proptest_command_passthrough_fidelity(
            cmd_id in "[a-z0-9-]{1,20}",
            action in prop_oneof![Just("lock".to_string()), Just("unlock".to_string())],
            doors in prop::collection::vec("[a-z]{1,10}".prop_map(|s| s), 1..5),
        ) {
            use crate::models::CommandPayload;

            // Build a valid command payload with extra fields (to verify
            // passthrough of fields the validator does not inspect).
            let payload = serde_json::json!({
                "command_id": cmd_id,
                "action": action,
                "doors": doors,
                "source": "companion_app",
                "timestamp": 1700000000u64,
            });
            let payload_str = serde_json::to_string(&payload).unwrap();
            let payload_bytes = payload_str.as_bytes();

            // Validate the payload.
            let result = validate_command_payload(payload_bytes);
            prop_assert!(result.is_ok(), "valid payload should pass validation");

            // Property: the original bytes are unchanged after validation.
            // The validator borrows the slice; it does not mutate or consume it.
            let reparsed: serde_json::Value =
                serde_json::from_slice(payload_bytes).unwrap();
            prop_assert_eq!(&payload, &reparsed, "payload bytes unchanged after validation");

            // Property: the parsed CommandPayload preserves all original fields,
            // including extra fields via serde(flatten).
            let cmd: CommandPayload = serde_json::from_slice(payload_bytes).unwrap();
            prop_assert_eq!(&cmd.command_id, &cmd_id);
            prop_assert_eq!(&cmd.action, &action);
            prop_assert_eq!(&cmd.doors, &doors);
            prop_assert!(
                cmd.extra.contains_key("source"),
                "extra fields should be preserved via serde(flatten)"
            );
        }
    }

    // TS-04-P4: Response Relay Fidelity
    //
    // For any JSON written to Vehicle.Command.Door.Response in DATA_BROKER,
    // the same JSON is published verbatim to NATS. At the unit level, we
    // verify that valid response JSON round-trips through serde without
    // modification — the relay code publishes the string as-is without
    // parsing it into a struct. The end-to-end property is tested by
    // TS-04-11 in tests/integration_tests.rs.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn proptest_response_relay_fidelity(
            cmd_id in "[a-z0-9-]{1,20}",
            status in prop_oneof![Just("success".to_string()), Just("failed".to_string())],
            timestamp in 1_000_000_000u64..2_000_000_000u64,
        ) {
            // Build a response JSON string.
            let response = serde_json::json!({
                "command_id": cmd_id,
                "status": status,
                "timestamp": timestamp,
            });
            let response_str = serde_json::to_string(&response).unwrap();

            // Property: the JSON string is valid and round-trips identically.
            // The relay code validates JSON (REQ-7.E1) but does not modify it.
            let parsed: serde_json::Value =
                serde_json::from_str(&response_str).unwrap();
            prop_assert_eq!(&response, &parsed, "JSON should round-trip unchanged");

            // Property: required fields are present (REQ-7.2).
            prop_assert!(parsed.get("command_id").is_some(), "command_id present");
            prop_assert!(parsed.get("status").is_some(), "status present");
            prop_assert!(parsed.get("timestamp").is_some(), "timestamp present");

            // Property: serialization produces a stable string.
            let roundtrip = serde_json::to_string(&parsed).unwrap();
            prop_assert_eq!(
                &response_str, &roundtrip,
                "JSON serialization should be stable for verbatim relay"
            );
        }
    }

    // TS-04-P6: Startup Determinism
    //
    // For any failure at step N in the startup sequence, steps 1..N-1
    // complete, step N fails, and steps N+1..end do not execute.
    //
    // At the unit level, we verify step 1 (config): a missing VIN produces
    // a ConfigError that the main function uses for early exit. Integration
    // tests verify the remaining steps:
    // - TS-04-SMOKE-2: missing VIN exits with code 1
    // - TS-04-15: NATS unreachable exits with code 1 after retries
    // - test_startup_exits_on_unreachable_broker: DATA_BROKER unreachable
    //   prevents registration and exits with code 1
    #[test]
    fn proptest_startup_determinism() {
        use crate::config::Config;
        use crate::errors::ConfigError;

        // Step 1 failure: missing VIN prevents all subsequent steps.
        // Save and restore env to avoid test interference.
        let saved_vin = std::env::var("VIN").ok();
        std::env::remove_var("VIN");

        let result = Config::from_env();
        assert!(result.is_err(), "missing VIN should produce error");
        assert_eq!(
            result.unwrap_err(),
            ConfigError::MissingVin,
            "error should be MissingVin"
        );

        // Restore VIN if it was set.
        if let Some(v) = saved_vin {
            std::env::set_var("VIN", v);
        }

        // The startup ordering invariant (config -> NATS -> DATA_BROKER ->
        // registration -> processing) is enforced by sequential early-return
        // logic in main.rs. The integration tests verify that failures at
        // steps 2 (NATS) and 3 (DATA_BROKER) produce exit code 1 without
        // reaching subsequent steps.
    }
}
