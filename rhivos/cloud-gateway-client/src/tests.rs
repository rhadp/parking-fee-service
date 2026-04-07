/// Unit and property tests for cloud-gateway-client.
///
/// Task Group 1: All tests are expected to FAIL until implementation is complete.

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
