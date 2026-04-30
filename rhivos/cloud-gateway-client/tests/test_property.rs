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
/// For any valid command payload, verify that the bytes written to
/// `Vehicle.Command.Door.Lock` in DATA_BROKER are identical to the
/// original NATS message payload.
///
/// Requires running NATS and DATA_BROKER containers.
#[tokio::test]
#[ignore]
async fn prop_command_passthrough_fidelity() {
    use cloud_gateway_client::broker_client::kuksa::val::v2;
    use std::time::Duration;

    let vin = "PROP-PASS-VIN";

    // Build and start the service.
    let build_output = tokio::process::Command::new("cargo")
        .args(["build", "-p", "cloud-gateway-client"])
        .output()
        .await
        .expect("Failed to run cargo build");
    assert!(build_output.status.success(), "cargo build failed");

    let metadata_output = tokio::process::Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .await
        .expect("Failed to run cargo metadata");
    let meta: serde_json::Value =
        serde_json::from_slice(&metadata_output.stdout).expect("Invalid cargo metadata JSON");
    let target_dir = meta["target_directory"]
        .as_str()
        .expect("Missing target_directory");
    let binary = format!("{target_dir}/debug/cloud-gateway-client");

    let mut child = tokio::process::Command::new(&binary)
        .env("VIN", vin)
        .env("NATS_URL", "nats://localhost:4222")
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .env("BEARER_TOKEN", "demo-token")
        .env("RUST_LOG", "cloud_gateway_client=debug")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to start service: {e}"));

    tokio::time::sleep(Duration::from_secs(3)).await;

    let nats = async_nats::connect("nats://localhost:4222")
        .await
        .expect("Failed to connect to NATS");

    // Test several payloads with varying extra fields to verify passthrough.
    let payloads = vec![
        r#"{"command_id":"pass-1","action":"lock","doors":["driver"]}"#,
        r#"{"command_id":"pass-2","action":"unlock","doors":["driver","passenger"],"extra_field":"preserved"}"#,
        r#"{"command_id":"pass-3","action":"lock","doors":[],"source":"app","vin":"X","timestamp":42}"#,
    ];

    let mut broker =
        v2::val_client::ValClient::connect("http://localhost:55556".to_string())
            .await
            .expect("Failed to connect to DATA_BROKER");

    for payload in &payloads {
        let subject = format!("vehicles.{vin}.commands");
        let mut headers = async_nats::HeaderMap::new();
        headers.insert("Authorization", "Bearer demo-token");

        nats.publish_with_headers(subject, headers, payload.to_string().into())
            .await
            .expect("Failed to publish command");
        nats.flush().await.expect("Failed to flush");

        tokio::time::sleep(Duration::from_secs(1)).await;

        let request = v2::GetValueRequest {
            signal_id: Some(v2::SignalId {
                signal: Some(v2::signal_id::Signal::Path(
                    "Vehicle.Command.Door.Lock".to_string(),
                )),
            }),
        };

        let response = broker
            .get_value(request)
            .await
            .expect("Failed to read from DATA_BROKER");

        let value = response
            .into_inner()
            .data_point
            .and_then(|dp| dp.value)
            .and_then(|v| {
                if let Some(v2::value::TypedValue::String(s)) = v.typed_value {
                    Some(s)
                } else {
                    None
                }
            });

        assert_eq!(
            value.as_deref(),
            Some(*payload),
            "DATA_BROKER value should be identical to original payload"
        );
    }

    child.kill().await.ok();
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
/// For any valid JSON string written to `Vehicle.Command.Door.Response`,
/// verify the NATS message on `vehicles.{VIN}.command_responses` contains
/// the identical bytes.
///
/// Requires running NATS and DATA_BROKER containers.
#[tokio::test]
#[ignore]
async fn prop_response_relay_fidelity() {
    use cloud_gateway_client::broker_client::kuksa::val::v2;
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::timeout;

    let vin = "PROP-RELAY-VIN";

    // Build and start the service.
    let build_output = tokio::process::Command::new("cargo")
        .args(["build", "-p", "cloud-gateway-client"])
        .output()
        .await
        .expect("Failed to run cargo build");
    assert!(build_output.status.success(), "cargo build failed");

    let metadata_output = tokio::process::Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .await
        .expect("Failed to run cargo metadata");
    let meta: serde_json::Value =
        serde_json::from_slice(&metadata_output.stdout).expect("Invalid cargo metadata JSON");
    let target_dir = meta["target_directory"]
        .as_str()
        .expect("Missing target_directory");
    let binary = format!("{target_dir}/debug/cloud-gateway-client");

    let mut child = tokio::process::Command::new(&binary)
        .env("VIN", vin)
        .env("NATS_URL", "nats://localhost:4222")
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .env("BEARER_TOKEN", "demo-token")
        .env("RUST_LOG", "cloud_gateway_client=debug")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to start service: {e}"));

    tokio::time::sleep(Duration::from_secs(3)).await;

    let nats = async_nats::connect("nats://localhost:4222")
        .await
        .expect("Failed to connect to NATS");

    let response_subject = format!("vehicles.{vin}.command_responses");
    let mut sub = nats
        .subscribe(response_subject)
        .await
        .expect("Failed to subscribe to command_responses");

    let mut broker =
        v2::val_client::ValClient::connect("http://localhost:55556".to_string())
            .await
            .expect("Failed to connect to DATA_BROKER");

    // Test several response payloads to verify verbatim relay.
    let responses = vec![
        r#"{"command_id":"rsp-1","status":"success","timestamp":1700000001}"#,
        r#"{"command_id":"rsp-2","status":"failed","reason":"door_jammed","timestamp":1700000002}"#,
        r#"{"command_id":"rsp-3","status":"success","timestamp":1700000003,"extra":"field"}"#,
    ];

    for response_json in &responses {
        let request = v2::PublishValueRequest {
            signal_id: Some(v2::SignalId {
                signal: Some(v2::signal_id::Signal::Path(
                    "Vehicle.Command.Door.Response".to_string(),
                )),
            }),
            data_point: Some(v2::Datapoint {
                timestamp: None,
                value: Some(v2::Value {
                    typed_value: Some(v2::value::TypedValue::String(
                        response_json.to_string(),
                    )),
                }),
            }),
        };

        broker
            .publish_value(request)
            .await
            .expect("Failed to write response to DATA_BROKER");

        let msg = timeout(Duration::from_secs(5), sub.next())
            .await
            .expect("Timed out waiting for relayed response")
            .expect("Subscription ended without receiving a response");

        let received = std::str::from_utf8(&msg.payload).expect("Response not valid UTF-8");
        assert_eq!(
            received, *response_json,
            "Response should be relayed verbatim"
        );
    }

    child.kill().await.ok();
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
/// For each failure step in [config, nats_connect, broker_connect], verify:
/// 1. Steps before the failure step complete
/// 2. The service exits with a non-zero code
/// 3. Steps after the failure step do not execute
///
/// Requires running NATS and DATA_BROKER containers (for the broker_connect
/// failure case, NATS must succeed first).
#[tokio::test]
#[ignore]
async fn prop_startup_determinism() {
    // Build the service binary.
    let build_output = tokio::process::Command::new("cargo")
        .args(["build", "-p", "cloud-gateway-client"])
        .output()
        .await
        .expect("Failed to run cargo build");
    assert!(build_output.status.success(), "cargo build failed");

    let metadata_output = tokio::process::Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .await
        .expect("Failed to run cargo metadata");
    let meta: serde_json::Value =
        serde_json::from_slice(&metadata_output.stdout).expect("Invalid cargo metadata JSON");
    let target_dir = meta["target_directory"]
        .as_str()
        .expect("Missing target_directory");
    let binary = format!("{target_dir}/debug/cloud-gateway-client");

    // --- Case 1: Config failure (VIN missing) ---
    // Steps: config fails -> no NATS connect, no broker connect, no registration.
    {
        let output = tokio::process::Command::new(&binary)
            .env_remove("VIN")
            .env("NATS_URL", "nats://localhost:4222")
            .env("DATABROKER_ADDR", "http://localhost:55556")
            .env("RUST_LOG", "cloud_gateway_client=debug")
            .output()
            .await
            .expect("Failed to start service");

        assert_eq!(
            output.status.code(),
            Some(1),
            "Config failure should exit with code 1"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should NOT see "Connected to NATS" (step 2 did not run).
        assert!(
            !stderr.contains("Connected to NATS"),
            "NATS connection should not be attempted after config failure"
        );
    }

    // --- Case 2: NATS failure (unreachable URL) ---
    // Steps: config succeeds, NATS connect fails -> no broker connect, no registration.
    {
        let output = tokio::process::Command::new(&binary)
            .env("VIN", "STARTUP-DET-VIN")
            .env("NATS_URL", "nats://127.0.0.1:19999")
            .env("DATABROKER_ADDR", "http://localhost:55556")
            .env("RUST_LOG", "cloud_gateway_client=debug")
            .output()
            .await
            .expect("Failed to start service");

        assert_eq!(
            output.status.code(),
            Some(1),
            "NATS failure should exit with code 1"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Configuration loaded") || stderr.contains("NATS connection failed"),
            "Config step should complete before NATS failure"
        );
        // Should NOT see "Connected to DATA_BROKER" (step 3 did not run).
        assert!(
            !stderr.contains("Connected to DATA_BROKER"),
            "DATA_BROKER connection should not be attempted after NATS failure"
        );
    }

    // --- Case 3: DATA_BROKER failure (unreachable address, NATS running) ---
    // Steps: config succeeds, NATS succeeds, broker connect fails -> no registration.
    {
        let output = tokio::process::Command::new(&binary)
            .env("VIN", "STARTUP-DET-VIN2")
            .env("NATS_URL", "nats://localhost:4222")
            .env("DATABROKER_ADDR", "http://127.0.0.1:19998")
            .env("RUST_LOG", "cloud_gateway_client=debug")
            .output()
            .await
            .expect("Failed to start service");

        assert_eq!(
            output.status.code(),
            Some(1),
            "DATA_BROKER failure should exit with code 1"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Connected to NATS"),
            "NATS should connect before DATA_BROKER failure"
        );
        // Should NOT see "Self-registration published" (step 4 did not run).
        assert!(
            !stderr.contains("Self-registration published"),
            "Registration should not happen after DATA_BROKER failure"
        );
    }
}
