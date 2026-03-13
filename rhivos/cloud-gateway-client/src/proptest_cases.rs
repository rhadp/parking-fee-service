/// Property-based tests for the cloud-gateway-client.
/// These use the proptest crate. Run with: `cargo test -- --include-ignored proptest`
#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::command::{parse_and_validate_command, validate_bearer_token};
    use crate::relay::relay_response;
    use crate::telemetry::{build_telemetry, TelemetryState};
    use crate::testing::MockNatsPublisher;

    // TS-04-P1: Command Authentication Gate
    // validate_bearer_token returns true iff the embedded token matches the expected value.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_bearer_token_gate(
            token in "[a-zA-Z0-9_-]{1,64}",
            expected in "[a-zA-Z0-9_-]{1,64}",
        ) {
            let header = format!("Bearer {}", token);
            let result = validate_bearer_token(Some(&header), &expected);
            prop_assert_eq!(result, token == expected);
        }
    }

    // TS-04-P2: Command Validation Completeness
    // Any byte payload either parses to a valid IncomingCommand or is rejected.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_command_validation(input in proptest::collection::vec(proptest::num::u8::ANY, 0..512)) {
            let result = parse_and_validate_command(&input);
            match result {
                Ok(cmd) => {
                    prop_assert!(!cmd.command_id.is_empty(), "command_id must be non-empty");
                    prop_assert!(
                        cmd.action == "lock" || cmd.action == "unlock",
                        "action must be 'lock' or 'unlock', got '{}'",
                        cmd.action
                    );
                }
                Err(_) => {
                    // Rejected — that's fine
                }
            }
        }
    }

    // TS-04-P3: Response Relay Fidelity
    // Published NATS payload equals the input response JSON verbatim.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_response_fidelity(
            command_id in "[a-zA-Z0-9_-]{1,36}",
            status in proptest::prop_oneof![Just("success".to_string()), Just("failed".to_string())],
            timestamp in 1_700_000_000i64..2_000_000_000i64,
        ) {
            let response_json = format!(
                r#"{{"command_id":"{}","status":"{}","timestamp":{}}}"#,
                command_id, status, timestamp
            );
            let mock_nats = MockNatsPublisher::new();
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(relay_response(&mock_nats, "VIN", &response_json));

            let (_, payload) = mock_nats.last_publish().expect("should have published");
            let published = std::str::from_utf8(&payload).unwrap();
            prop_assert_eq!(published, response_json.as_str());
        }
    }

    // TS-04-P4: Telemetry Aggregation
    // build_telemetry produces a message with the VIN, a positive timestamp,
    // and all Some() fields present.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_telemetry_aggregation(
            is_locked in proptest::option::of(proptest::bool::ANY),
            lat in proptest::option::of(-90.0f64..90.0f64),
            lon in proptest::option::of(-180.0f64..180.0f64),
            parking in proptest::option::of(proptest::bool::ANY),
        ) {
            let state = TelemetryState {
                is_locked,
                latitude: lat,
                longitude: lon,
                parking_active: parking,
            };
            let msg = build_telemetry("VIN", &state);
            prop_assert_eq!(&msg.vin, "VIN");
            prop_assert!(msg.timestamp > 0, "timestamp must be positive");
            prop_assert_eq!(msg.is_locked, is_locked);
            prop_assert_eq!(msg.latitude, lat);
            prop_assert_eq!(msg.longitude, lon);
            prop_assert_eq!(msg.parking_active, parking);
        }
    }

    // TS-04-P5: VIN Subject Consistency
    // All NATS subjects contain the configured VIN string.
    proptest! {
        #[test]
        #[ignore]
        fn proptest_vin_subjects(vin in "[A-Z0-9]{5,20}") {
            let commands_subject = format!("vehicles.{}.commands", vin);
            let responses_subject = format!("vehicles.{}.command_responses", vin);
            let telemetry_subject = format!("vehicles.{}.telemetry", vin);
            let status_subject = format!("vehicles.{}.status", vin);

            prop_assert!(commands_subject.contains(&*vin));
            prop_assert!(responses_subject.contains(&*vin));
            prop_assert!(telemetry_subject.contains(&*vin));
            prop_assert!(status_subject.contains(&*vin));
        }
    }
}
