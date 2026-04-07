/// Unit, property, and integration tests for cloud-gateway-client.
///
/// Unit/property tests run without infrastructure.
/// Integration tests (marked `#[ignore]`) require NATS and DATA_BROKER containers.

#[cfg(test)]
mod config_tests {
    use crate::config::Config;
    use crate::errors::ConfigError;

    /// TS-04-1: Config reads VIN from environment
    /// Validates: [04-REQ-1.1]
    #[test]
    fn ts_04_1_config_reads_vin_from_env() {
        // Clean env
        std::env::remove_var("NATS_URL");
        std::env::remove_var("DATABROKER_ADDR");
        std::env::remove_var("BEARER_TOKEN");
        std::env::set_var("VIN", "TEST-VIN-001");

        let config = Config::from_env().expect("Config::from_env should succeed when VIN is set");

        assert_eq!(config.vin, "TEST-VIN-001");
        assert_eq!(config.nats_url, "nats://localhost:4222");
        assert_eq!(config.databroker_addr, "http://localhost:55556");
        assert_eq!(config.bearer_token, "demo-token");

        // Clean up
        std::env::remove_var("VIN");
    }

    /// TS-04-2: Config reads all custom environment variables
    /// Validates: [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4]
    #[test]
    fn ts_04_2_config_reads_all_custom_env_vars() {
        std::env::set_var("VIN", "MY-VIN");
        std::env::set_var("NATS_URL", "nats://custom:9222");
        std::env::set_var("DATABROKER_ADDR", "http://custom:55557");
        std::env::set_var("BEARER_TOKEN", "secret-token");

        let config = Config::from_env().expect("Config::from_env should succeed");

        assert_eq!(config.vin, "MY-VIN");
        assert_eq!(config.nats_url, "nats://custom:9222");
        assert_eq!(config.databroker_addr, "http://custom:55557");
        assert_eq!(config.bearer_token, "secret-token");

        // Clean up
        std::env::remove_var("VIN");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("DATABROKER_ADDR");
        std::env::remove_var("BEARER_TOKEN");
    }

    /// TS-04-E1: Config fails when VIN is missing
    /// Validates: [04-REQ-1.E1]
    #[test]
    fn ts_04_e1_config_fails_when_vin_missing() {
        std::env::remove_var("VIN");

        let result = Config::from_env();

        assert!(result.is_err(), "Config::from_env should fail when VIN is not set");
        assert_eq!(result.unwrap_err(), ConfigError::MissingVin);
    }
}

#[cfg(test)]
mod bearer_token_tests {
    use crate::command_validator::validate_bearer_token;
    use crate::errors::AuthError;

    /// TS-04-3: Bearer token validation accepts valid token
    /// Validates: [04-REQ-5.1], [04-REQ-5.2]
    #[test]
    fn ts_04_3_bearer_token_valid() {
        let result = validate_bearer_token(Some("Bearer demo-token"), "demo-token");
        assert!(result.is_ok(), "Valid bearer token should be accepted");
    }

    /// TS-04-E2: Bearer token validation rejects missing header
    /// Validates: [04-REQ-5.E1]
    #[test]
    fn ts_04_e2_bearer_token_missing_header() {
        let result = validate_bearer_token(None, "demo-token");
        assert!(result.is_err(), "Missing header should be rejected");
        assert_eq!(result.unwrap_err(), AuthError::MissingHeader);
    }

    /// TS-04-E3: Bearer token validation rejects wrong token
    /// Validates: [04-REQ-5.E2]
    #[test]
    fn ts_04_e3_bearer_token_wrong_token() {
        let result = validate_bearer_token(Some("Bearer wrong-token"), "demo-token");
        assert!(result.is_err(), "Wrong token should be rejected");
        assert_eq!(result.unwrap_err(), AuthError::InvalidToken);
    }

    /// TS-04-E4: Bearer token validation rejects malformed header
    /// Validates: [04-REQ-5.E2]
    #[test]
    fn ts_04_e4_bearer_token_malformed_header() {
        let result = validate_bearer_token(Some("NotBearer demo-token"), "demo-token");
        assert!(result.is_err(), "Malformed header should be rejected");
        assert_eq!(result.unwrap_err(), AuthError::InvalidToken);
    }
}

#[cfg(test)]
mod command_payload_tests {
    use crate::command_validator::validate_command_payload;
    use crate::errors::ValidationError;

    /// TS-04-4: Command validation accepts valid payload
    /// Validates: [04-REQ-6.1], [04-REQ-6.2]
    #[test]
    fn ts_04_4_valid_command_payload() {
        let payload = r#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_ok(), "Valid command payload should be accepted");
        let cmd = result.unwrap();
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, "lock");
        assert_eq!(cmd.doors, vec!["driver"]);
    }

    /// TS-04-5: Command validation accepts unlock action
    /// Validates: [04-REQ-6.2]
    #[test]
    fn ts_04_5_valid_unlock_action() {
        let payload = r#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_ok(), "Unlock action should be accepted");
        let cmd = result.unwrap();
        assert_eq!(cmd.action, "unlock");
    }

    /// TS-04-E5: Command validation rejects invalid JSON
    /// Validates: [04-REQ-6.E1]
    #[test]
    fn ts_04_e5_invalid_json() {
        let payload = b"not-valid-json{{";
        let result = validate_command_payload(payload);
        assert!(result.is_err(), "Invalid JSON should be rejected");
        match result.unwrap_err() {
            ValidationError::InvalidJson(_) => {} // expected
            other => panic!("Expected InvalidJson, got: {:?}", other),
        }
    }

    /// TS-04-E6: Command validation rejects missing command_id
    /// Validates: [04-REQ-6.E2]
    #[test]
    fn ts_04_e6_missing_command_id() {
        let payload = r#"{"action":"lock","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err(), "Missing command_id should be rejected");
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("command_id".to_string())
        );
    }

    /// TS-04-E7: Command validation rejects empty command_id
    /// Validates: [04-REQ-6.E2]
    #[test]
    fn ts_04_e7_empty_command_id() {
        let payload = r#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err(), "Empty command_id should be rejected");
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("command_id".to_string())
        );
    }

    /// TS-04-E8: Command validation rejects missing action
    /// Validates: [04-REQ-6.E2]
    #[test]
    fn ts_04_e8_missing_action() {
        let payload = r#"{"command_id":"abc","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err(), "Missing action should be rejected");
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("action".to_string())
        );
    }

    /// TS-04-E9: Command validation rejects invalid action
    /// Validates: [04-REQ-6.E3]
    #[test]
    fn ts_04_e9_invalid_action() {
        let payload = r#"{"command_id":"abc","action":"open","doors":["driver"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err(), "Invalid action should be rejected");
        assert_eq!(
            result.unwrap_err(),
            ValidationError::InvalidAction("open".to_string())
        );
    }

    /// TS-04-E10: Command validation rejects missing doors
    /// Validates: [04-REQ-6.E2]
    #[test]
    fn ts_04_e10_missing_doors() {
        let payload = r#"{"command_id":"abc","action":"lock"}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_err(), "Missing doors should be rejected");
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("doors".to_string())
        );
    }

    /// TS-04-6: Command validation does not validate door values
    /// Validates: [04-REQ-6.4]
    #[test]
    fn ts_04_6_does_not_validate_door_values() {
        let payload = r#"{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}"#;
        let result = validate_command_payload(payload.as_bytes());
        assert!(result.is_ok(), "Unknown door values should be accepted");
        let cmd = result.unwrap();
        assert_eq!(cmd.doors, vec!["unknown-door", "another"]);
    }
}

#[cfg(test)]
mod telemetry_tests {
    use crate::models::SignalUpdate;
    use crate::telemetry::TelemetryState;

    /// TS-04-7: Telemetry state produces JSON on first update
    /// Validates: [04-REQ-8.1], [04-REQ-8.2]
    #[test]
    fn ts_04_7_telemetry_first_update() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        let result = state.update(SignalUpdate::IsLocked(true));

        assert!(result.is_some(), "First update should produce JSON");
        let json = result.unwrap();

        assert!(json.contains(r#""vin":"VIN-001""#), "JSON should contain vin");
        assert!(json.contains(r#""is_locked":true"#), "JSON should contain is_locked");
        assert!(json.contains("\"timestamp\""), "JSON should contain timestamp");
        assert!(!json.contains("\"latitude\""), "JSON should not contain latitude");
        assert!(!json.contains("\"longitude\""), "JSON should not contain longitude");
        assert!(
            !json.contains("\"parking_active\""),
            "JSON should not contain parking_active"
        );
    }

    /// TS-04-8: Telemetry state omits unset fields
    /// Validates: [04-REQ-8.3]
    #[test]
    fn ts_04_8_telemetry_omits_unset_fields() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        let result = state.update(SignalUpdate::Latitude(48.1351));

        assert!(result.is_some(), "Update should produce JSON");
        let json = result.unwrap();

        assert!(json.contains("48.1351"), "JSON should contain latitude value");
        assert!(!json.contains("\"is_locked\""), "JSON should not contain is_locked");
        assert!(!json.contains("\"longitude\""), "JSON should not contain longitude");
        assert!(
            !json.contains("\"parking_active\""),
            "JSON should not contain parking_active"
        );
    }

    /// TS-04-9: Telemetry state includes all known fields
    /// Validates: [04-REQ-8.2]
    #[test]
    fn ts_04_9_telemetry_includes_all_known_fields() {
        let mut state = TelemetryState::new("VIN-001".to_string());

        state.update(SignalUpdate::IsLocked(true));
        state.update(SignalUpdate::Latitude(48.1351));
        state.update(SignalUpdate::Longitude(11.582));
        let result = state.update(SignalUpdate::ParkingActive(true));

        assert!(result.is_some(), "Update should produce JSON");
        let json = result.unwrap();

        assert!(json.contains(r#""is_locked":true"#), "JSON should contain is_locked");
        assert!(json.contains("48.1351"), "JSON should contain latitude");
        assert!(json.contains("11.582"), "JSON should contain longitude");
        assert!(
            json.contains(r#""parking_active":true"#),
            "JSON should contain parking_active"
        );
    }
}

#[cfg(test)]
mod registration_tests {
    use crate::models::RegistrationMessage;

    /// TS-04-P1: Registration message format
    /// Validates: [04-REQ-4.1]
    #[test]
    fn ts_04_p1_registration_message_format() {
        let msg = RegistrationMessage {
            vin: "VIN-001".to_string(),
            status: "online".to_string(),
            timestamp: 1700000000,
        };

        let json = serde_json::to_string(&msg).expect("Serialization should succeed");

        assert!(
            json.contains(r#""vin":"VIN-001""#),
            "JSON should contain vin"
        );
        assert!(
            json.contains(r#""status":"online""#),
            "JSON should contain status"
        );
        assert!(json.contains("\"timestamp\""), "JSON should contain timestamp");
    }
}

#[cfg(test)]
mod property_tests {
    use crate::command_validator::validate_command_payload;
    use crate::models::SignalUpdate;
    use crate::telemetry::TelemetryState;
    use proptest::prelude::*;

    /// TS-04-P2: Command Structural Validity (property test)
    /// Validates: [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3]
    ///
    /// For any command that passes authentication, the system writes to DATA_BROKER
    /// only if the payload is valid JSON containing a non-empty command_id,
    /// an action of "lock" or "unlock", and a doors array.
    #[test]
    fn ts_04_p2_valid_commands_always_accepted() {
        // Generate valid command payloads and verify they pass validation.
        let test_cases = vec![
            r#"{"command_id":"id-1","action":"lock","doors":[]}"#,
            r#"{"command_id":"id-2","action":"unlock","doors":["a"]}"#,
            r#"{"command_id":"x","action":"lock","doors":["a","b","c"]}"#,
        ];

        for payload in test_cases {
            let result = validate_command_payload(payload.as_bytes());
            assert!(
                result.is_ok(),
                "Valid payload should be accepted: {}",
                payload
            );
        }
    }

    #[test]
    fn ts_04_p2_invalid_commands_always_rejected() {
        // Various invalid payloads should all be rejected.
        let test_cases: Vec<(&str, &str)> = vec![
            ("not json", "invalid json"),
            (r#"{"action":"lock","doors":[]}"#, "missing command_id"),
            (r#"{"command_id":"","action":"lock","doors":[]}"#, "empty command_id"),
            (r#"{"command_id":"id","doors":[]}"#, "missing action"),
            (r#"{"command_id":"id","action":"open","doors":[]}"#, "invalid action"),
            (r#"{"command_id":"id","action":"lock"}"#, "missing doors"),
        ];

        for (payload, desc) in test_cases {
            let result = validate_command_payload(payload.as_bytes());
            assert!(
                result.is_err(),
                "Invalid payload should be rejected ({}): {}",
                desc,
                payload
            );
        }
    }

    // TS-04-P2: Property-based test for command validation
    proptest! {
        #[test]
        fn ts_04_p2_arbitrary_bytes_never_panic(payload in proptest::collection::vec(any::<u8>(), 0..256)) {
            // Validation should never panic, regardless of input.
            let _ = validate_command_payload(&payload);
        }
    }

    /// TS-04-P5: Telemetry Completeness (property test)
    /// Validates: [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
    ///
    /// For any sequence of signal updates, the published JSON includes all
    /// previously updated fields and omits signals never set.
    #[test]
    fn ts_04_p5_telemetry_completeness() {
        let updates = vec![
            SignalUpdate::IsLocked(false),
            SignalUpdate::Latitude(52.52),
            SignalUpdate::Longitude(13.405),
            SignalUpdate::ParkingActive(true),
        ];

        let mut state = TelemetryState::new("PROP-VIN".to_string());
        let mut known_fields: Vec<&str> = Vec::new();

        for update in &updates {
            let field_name = match update {
                SignalUpdate::IsLocked(_) => "is_locked",
                SignalUpdate::Latitude(_) => "latitude",
                SignalUpdate::Longitude(_) => "longitude",
                SignalUpdate::ParkingActive(_) => "parking_active",
            };
            let result = state.update(update.clone());
            known_fields.push(field_name);

            assert!(
                result.is_some(),
                "Update for {} should produce JSON",
                field_name
            );
            let json = result.unwrap();

            // All known fields should be present
            for known in &known_fields {
                assert!(
                    json.contains(&format!("\"{}\"", known)),
                    "JSON should contain field '{}' after it was set. JSON: {}",
                    known,
                    json
                );
            }

            // Unknown fields should be absent
            let all_fields = ["is_locked", "latitude", "longitude", "parking_active"];
            for field in &all_fields {
                if !known_fields.contains(field) {
                    assert!(
                        !json.contains(&format!("\"{}\"", field)),
                        "JSON should not contain unset field '{}'. JSON: {}",
                        field,
                        json
                    );
                }
            }
        }
    }

    // TS-04-P5: Property-based telemetry test with proptest
    proptest! {
        #[test]
        fn ts_04_p5_telemetry_always_includes_vin(
            locked in any::<bool>(),
            lat in -90.0f64..90.0,
            lon in -180.0f64..180.0,
        ) {
            let mut state = TelemetryState::new("PROP-VIN".to_string());
            let result = state.update(SignalUpdate::IsLocked(locked));
            prop_assert!(result.is_some(), "Update should produce JSON");
            let json = result.unwrap();
            prop_assert!(json.contains(r#""vin":"PROP-VIN""#), "JSON should contain vin");

            let result = state.update(SignalUpdate::Latitude(lat));
            prop_assert!(result.is_some());
            let json = result.unwrap();
            prop_assert!(json.contains(r#""is_locked""#), "Should retain is_locked after lat update");

            let result = state.update(SignalUpdate::Longitude(lon));
            prop_assert!(result.is_some());
            let json = result.unwrap();
            prop_assert!(json.contains(r#""latitude""#), "Should retain latitude after lon update");
        }
    }
}

/// Integration tests requiring NATS and DATA_BROKER containers.
///
/// Run with: `cargo test -p cloud-gateway-client -- --ignored`
///
/// Prerequisites:
///   cd deployments && podman-compose up -d
#[cfg(test)]
mod integration_tests {
    use crate::broker_client::BrokerClient;
    use crate::command_validator::{validate_bearer_token, validate_command_payload};
    use crate::config::Config;
    use crate::nats_client::NatsClient;
    use crate::telemetry::TelemetryState;
    use futures_util::StreamExt;
    use std::time::Duration;

    /// Re-include the proto for direct gRPC helpers used by tests.
    #[allow(clippy::enum_variant_names)]
    #[allow(clippy::doc_lazy_continuation)]
    mod kuksa {
        tonic::include_proto!("kuksa.val.v2");
    }

    use kuksa::val_client::ValClient;

    /// Create a test configuration with the given VIN.
    fn test_config(vin: &str) -> Config {
        Config {
            vin: vin.to_string(),
            nats_url: "nats://localhost:4222".to_string(),
            databroker_addr: "http://localhost:55556".to_string(),
            bearer_token: "demo-token".to_string(),
        }
    }

    /// Helper: create a SignalID from a path string.
    fn signal_id(path: &str) -> Option<kuksa::SignalId> {
        Some(kuksa::SignalId {
            signal: Some(kuksa::signal_id::Signal::Path(path.to_string())),
        })
    }

    /// Helper: connect a raw gRPC client to DATA_BROKER for test setup/verification.
    async fn broker_grpc_client() -> ValClient<tonic::transport::Channel> {
        ValClient::connect("http://localhost:55556")
            .await
            .expect("Failed to connect gRPC client to DATA_BROKER for test")
    }

    /// Helper: read a string signal from DATA_BROKER.
    async fn get_broker_string(
        client: &mut ValClient<tonic::transport::Channel>,
        path: &str,
    ) -> Option<String> {
        let resp = client
            .get_value(kuksa::GetValueRequest {
                signal_id: signal_id(path),
            })
            .await
            .expect("gRPC get_value failed");
        let dp = resp.into_inner().data_point?;
        let value = dp.value?;
        match value.typed_value? {
            kuksa::value::TypedValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Helper: set a string signal in DATA_BROKER.
    async fn set_broker_string(
        client: &mut ValClient<tonic::transport::Channel>,
        path: &str,
        value: &str,
    ) {
        client
            .publish_value(kuksa::PublishValueRequest {
                signal_id: signal_id(path),
                data_point: Some(kuksa::Datapoint {
                    timestamp: None,
                    value: Some(kuksa::Value {
                        typed_value: Some(kuksa::value::TypedValue::String(
                            value.to_string(),
                        )),
                    }),
                }),
            })
            .await
            .expect("gRPC publish_value (string) failed");
    }

    /// Helper: set a boolean signal in DATA_BROKER.
    async fn set_broker_bool(
        client: &mut ValClient<tonic::transport::Channel>,
        path: &str,
        value: bool,
    ) {
        client
            .publish_value(kuksa::PublishValueRequest {
                signal_id: signal_id(path),
                data_point: Some(kuksa::Datapoint {
                    timestamp: None,
                    value: Some(kuksa::Value {
                        typed_value: Some(kuksa::value::TypedValue::Bool(value)),
                    }),
                }),
            })
            .await
            .expect("gRPC publish_value (bool) failed");
    }

    /// Helper: clear a string signal in DATA_BROKER by writing an empty string.
    async fn clear_broker_string(
        client: &mut ValClient<tonic::transport::Channel>,
        path: &str,
    ) {
        let _ = client
            .publish_value(kuksa::PublishValueRequest {
                signal_id: signal_id(path),
                data_point: Some(kuksa::Datapoint {
                    timestamp: None,
                    value: Some(kuksa::Value {
                        typed_value: Some(kuksa::value::TypedValue::String(
                            String::new(),
                        )),
                    }),
                }),
            })
            .await;
    }

    /// TS-04-10: End-to-end command flow
    ///
    /// Validates: [04-REQ-5.2], [04-REQ-6.3], [04-REQ-2.3]
    ///
    /// Publishes a command to NATS with valid auth, verifies the command payload
    /// is written to Vehicle.Command.Door.Lock in DATA_BROKER.
    #[tokio::test]
    #[ignore]
    async fn ts_04_10_e2e_command_flow() {
        let vin = "E2E-VIN-10";
        let config = test_config(vin);

        // Clear any previous command value.
        let mut grpc = broker_grpc_client().await;
        clear_broker_string(&mut grpc, "Vehicle.Command.Door.Lock").await;

        // Connect the service components.
        let nats_client = NatsClient::connect(&config)
            .await
            .expect("NatsClient connect failed");
        let broker_client = BrokerClient::connect(&config)
            .await
            .expect("BrokerClient connect failed");

        // Subscribe to NATS commands subject.
        let mut cmd_sub = nats_client
            .subscribe_commands()
            .await
            .expect("subscribe_commands failed");

        // Spawn a task that processes exactly one command (mirrors main::command_loop).
        let bearer = config.bearer_token.clone();
        let broker = broker_client.clone();
        let cmd_handle = tokio::spawn(async move {
            if let Some(msg) = cmd_sub.next().await {
                let auth = msg
                    .headers
                    .as_ref()
                    .and_then(|h| h.get("Authorization"))
                    .map(|v| v.as_str());
                if validate_bearer_token(auth, &bearer).is_err() {
                    panic!("Bearer token validation unexpectedly failed in test");
                }
                let payload = msg.payload.as_ref();
                if validate_command_payload(payload).is_err() {
                    panic!("Command validation unexpectedly failed in test");
                }
                let payload_str =
                    std::str::from_utf8(payload).expect("payload should be valid UTF-8");
                broker
                    .write_command(payload_str)
                    .await
                    .expect("write_command failed");
            }
        });

        // Allow subscriber to become ready.
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Publish a command via a separate NATS connection.
        let raw_nats = async_nats::connect("nats://localhost:4222")
            .await
            .expect("raw NATS connect failed");
        let mut headers = async_nats::HeaderMap::new();
        headers.insert("Authorization", "Bearer demo-token");
        let command_payload = r#"{"command_id":"cmd-1","action":"lock","doors":["driver"],"source":"companion_app","vin":"E2E-VIN","timestamp":1700000000}"#;
        raw_nats
            .publish_with_headers(
                format!("vehicles.{}.commands", vin),
                headers,
                command_payload.into(),
            )
            .await
            .expect("NATS publish failed");
        raw_nats.flush().await.expect("NATS flush failed");

        // Wait for the command to be processed (max 2 seconds).
        tokio::time::timeout(Duration::from_secs(2), cmd_handle)
            .await
            .expect("Timeout waiting for command processing")
            .expect("Command processing task panicked");

        // Verify the command was written to DATA_BROKER.
        let stored = get_broker_string(&mut grpc, "Vehicle.Command.Door.Lock").await;
        assert!(stored.is_some(), "Command should be written to DATA_BROKER");
        assert_eq!(
            stored.unwrap(),
            command_payload,
            "DATA_BROKER should contain the command payload verbatim"
        );
    }

    /// TS-04-11: End-to-end response relay
    ///
    /// Validates: [04-REQ-7.1], [04-REQ-7.2]
    ///
    /// Sets Vehicle.Command.Door.Response in DATA_BROKER and verifies
    /// the response JSON is published verbatim to NATS.
    #[tokio::test]
    #[ignore]
    async fn ts_04_11_e2e_response_relay() {
        let vin = "E2E-VIN-11";
        let config = test_config(vin);

        // Connect the service components.
        let nats_client = NatsClient::connect(&config)
            .await
            .expect("NatsClient connect failed");
        let broker_client = BrokerClient::connect(&config)
            .await
            .expect("BrokerClient connect failed");

        // Subscribe to NATS response subject.
        let raw_nats = async_nats::connect("nats://localhost:4222")
            .await
            .expect("raw NATS connect failed");
        let mut response_sub = raw_nats
            .subscribe(format!("vehicles.{}.command_responses", vin))
            .await
            .expect("NATS subscribe failed");

        // Start the response relay: subscribe to DATA_BROKER responses,
        // then forward to NATS.  The v2 Subscribe API sends current values
        // immediately on subscription.  We forward all of them so we can
        // filter on the NATS side for the specific message we care about.
        let mut response_rx = broker_client
            .subscribe_responses()
            .await
            .expect("subscribe_responses failed");

        let nats_for_relay = nats_client.clone();
        let relay_handle = tokio::spawn(async move {
            // Forward multiple messages (initial value + actual update).
            while let Some(json) = response_rx.recv().await {
                nats_for_relay
                    .publish_response(&json)
                    .await
                    .expect("publish_response failed");
            }
        });

        // Allow subscriptions to become ready.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Write a response to DATA_BROKER.
        let response_json =
            r#"{"command_id":"cmd-1","status":"success","timestamp":1700000001}"#;
        let mut grpc = broker_grpc_client().await;
        set_broker_string(&mut grpc, "Vehicle.Command.Door.Response", response_json).await;

        // Verify the expected response was published to NATS.
        // The v2 API may deliver an initial value first, so we loop until
        // we find the expected message or time out.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut found = false;
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_secs(1), response_sub.next()).await {
                Ok(Some(nats_msg)) => {
                    let received =
                        std::str::from_utf8(&nats_msg.payload).expect("payload should be UTF-8");
                    if received == response_json {
                        found = true;
                        break;
                    }
                    // Skip stale/initial values
                }
                _ => break,
            }
        }
        assert!(found, "NATS should receive the response JSON verbatim");
        relay_handle.abort();
    }

    /// TS-04-12: End-to-end telemetry on signal change
    ///
    /// Validates: [04-REQ-8.1], [04-REQ-8.2]
    ///
    /// Sets Vehicle.Cabin.Door.Row1.DriverSide.IsLocked to true in DATA_BROKER
    /// and verifies a telemetry message is published to NATS.
    #[tokio::test]
    #[ignore]
    async fn ts_04_12_e2e_telemetry_on_signal_change() {
        let vin = "E2E-VIN-12";
        let config = test_config(vin);

        // Connect the service components.
        let nats_client = NatsClient::connect(&config)
            .await
            .expect("NatsClient connect failed");
        let broker_client = BrokerClient::connect(&config)
            .await
            .expect("BrokerClient connect failed");

        // Subscribe to NATS telemetry subject.
        let raw_nats = async_nats::connect("nats://localhost:4222")
            .await
            .expect("raw NATS connect failed");
        let mut telem_sub = raw_nats
            .subscribe(format!("vehicles.{}.telemetry", vin))
            .await
            .expect("NATS subscribe failed");

        // Start the telemetry loop: subscribe to DATA_BROKER signals,
        // aggregate via TelemetryState, forward to NATS.
        let mut telemetry_rx = broker_client
            .subscribe_telemetry()
            .await
            .expect("subscribe_telemetry failed");

        let nats_for_telem = nats_client.clone();
        let telem_vin = vin.to_string();
        let telem_handle = tokio::spawn(async move {
            let mut state = TelemetryState::new(telem_vin);
            // Process all incoming signal updates (initial + actual changes).
            while let Some(signal_update) = telemetry_rx.recv().await {
                if let Some(json) = state.update(signal_update) {
                    nats_for_telem
                        .publish_telemetry(&json)
                        .await
                        .expect("publish_telemetry failed");
                }
            }
        });

        // Allow subscriptions to become ready.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Set IsLocked to true in DATA_BROKER.
        let mut grpc = broker_grpc_client().await;
        set_broker_bool(
            &mut grpc,
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
            true,
        )
        .await;

        // Verify the telemetry was published to NATS.
        // The v2 API may deliver initial values first, so we loop until
        // we find a message with is_locked: true or time out.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut found = false;
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_secs(1), telem_sub.next()).await {
                Ok(Some(nats_msg)) => {
                    let json: serde_json::Value =
                        serde_json::from_slice(&nats_msg.payload)
                            .expect("telemetry should be valid JSON");
                    if json["vin"] == vin && json["is_locked"] == true {
                        found = true;
                        break;
                    }
                    // Skip telemetry from initial/stale values
                }
                _ => break,
            }
        }
        assert!(found, "NATS should receive telemetry with is_locked: true");
        telem_handle.abort();
    }

    /// TS-04-13: Self-registration on startup
    ///
    /// Validates: [04-REQ-4.1], [04-REQ-4.2]
    ///
    /// Verifies that NatsClient::publish_registration() sends a correctly
    /// formatted message to the status subject.
    #[tokio::test]
    #[ignore]
    async fn ts_04_13_self_registration_on_startup() {
        let vin = "REG-VIN-13";
        let config = test_config(vin);

        // Subscribe to the status subject before publishing.
        let raw_nats = async_nats::connect("nats://localhost:4222")
            .await
            .expect("raw NATS connect failed");
        let mut status_sub = raw_nats
            .subscribe(format!("vehicles.{}.status", vin))
            .await
            .expect("NATS subscribe failed");

        // Allow subscription to become ready.
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Connect the NatsClient and publish registration.
        let nats_client = NatsClient::connect(&config)
            .await
            .expect("NatsClient connect failed");
        nats_client
            .publish_registration()
            .await
            .expect("publish_registration failed");

        // Verify the registration message.
        let nats_msg = tokio::time::timeout(Duration::from_secs(5), status_sub.next())
            .await
            .expect("Timeout waiting for registration message")
            .expect("No registration message received");

        let json: serde_json::Value =
            serde_json::from_slice(&nats_msg.payload).expect("registration should be valid JSON");
        assert_eq!(
            json["vin"], vin,
            "Registration should contain correct VIN"
        );
        assert_eq!(
            json["status"], "online",
            "Registration should contain status: online"
        );
        assert!(
            json.get("timestamp").is_some(),
            "Registration should contain a timestamp"
        );
    }

    /// TS-04-14: Command rejected with invalid token
    ///
    /// Validates: [04-REQ-5.E2]
    ///
    /// Publishes a command with an invalid bearer token and verifies
    /// that DATA_BROKER is NOT updated.
    #[tokio::test]
    #[ignore]
    async fn ts_04_14_command_rejected_invalid_token() {
        let vin = "E2E-VIN-14";
        let config = test_config(vin);

        // Clear any previous command value.
        let mut grpc = broker_grpc_client().await;
        clear_broker_string(&mut grpc, "Vehicle.Command.Door.Lock").await;

        // Connect the service components.
        let nats_client = NatsClient::connect(&config)
            .await
            .expect("NatsClient connect failed");
        let _broker_client = BrokerClient::connect(&config)
            .await
            .expect("BrokerClient connect failed");

        // Subscribe to NATS commands subject.
        let mut cmd_sub = nats_client
            .subscribe_commands()
            .await
            .expect("subscribe_commands failed");

        // Track whether a write to DATA_BROKER was attempted.
        let broker_for_cmd = _broker_client.clone();
        let bearer = config.bearer_token.clone();
        let wrote_to_broker = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let wrote_flag = wrote_to_broker.clone();

        let cmd_handle = tokio::spawn(async move {
            if let Some(msg) = cmd_sub.next().await {
                let auth = msg
                    .headers
                    .as_ref()
                    .and_then(|h| h.get("Authorization"))
                    .map(|v| v.as_str());
                if validate_bearer_token(auth, &bearer).is_err() {
                    // Authentication failed; command is discarded (expected).
                    return;
                }
                let payload = msg.payload.as_ref();
                if validate_command_payload(payload).is_err() {
                    return;
                }
                let payload_str =
                    std::str::from_utf8(payload).expect("payload should be valid UTF-8");
                broker_for_cmd
                    .write_command(payload_str)
                    .await
                    .expect("write_command failed");
                wrote_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });

        // Allow subscriber to become ready.
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Publish a command with INVALID bearer token.
        let raw_nats = async_nats::connect("nats://localhost:4222")
            .await
            .expect("raw NATS connect failed");
        let mut headers = async_nats::HeaderMap::new();
        headers.insert("Authorization", "Bearer wrong-token");
        let command_payload =
            r#"{"command_id":"cmd-2","action":"lock","doors":["driver"]}"#;
        raw_nats
            .publish_with_headers(
                format!("vehicles.{}.commands", vin),
                headers,
                command_payload.into(),
            )
            .await
            .expect("NATS publish failed");
        raw_nats.flush().await.expect("NATS flush failed");

        // Wait for the command processing task to handle the message.
        tokio::time::timeout(Duration::from_secs(2), cmd_handle)
            .await
            .expect("Timeout waiting for command processing")
            .expect("Command processing task panicked");

        // Verify no write occurred to DATA_BROKER.
        assert!(
            !wrote_to_broker.load(std::sync::atomic::Ordering::SeqCst),
            "DATA_BROKER should NOT be updated when the bearer token is invalid"
        );

        // Also verify via gRPC that the command signal is empty/unchanged.
        let stored = get_broker_string(&mut grpc, "Vehicle.Command.Door.Lock").await;
        let is_empty = stored.as_ref().map_or(true, |s| s.is_empty());
        assert!(
            is_empty,
            "Vehicle.Command.Door.Lock should not contain a command payload"
        );
    }
}
