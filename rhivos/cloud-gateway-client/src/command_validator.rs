use crate::errors::{AuthError, ValidationError};
use crate::models::CommandPayload;

/// Validate the `Authorization` header from an incoming NATS message.
///
/// `authorization` is the raw value of the `Authorization` header, or `None`
/// when the header is absent.
///
/// Returns `Ok(())` when the header value exactly matches
/// `"Bearer <expected_token>"`.
pub fn validate_bearer_token(
    authorization: Option<&str>,
    expected_token: &str,
) -> Result<(), AuthError> {
    match authorization {
        None => Err(AuthError::MissingHeader),
        Some(header) => {
            let expected = format!("Bearer {}", expected_token);
            if header == expected {
                Ok(())
            } else {
                Err(AuthError::InvalidToken)
            }
        }
    }
}

/// Validate the raw bytes of an inbound command payload.
///
/// Checks (in order):
/// 1. Payload is valid JSON.
/// 2. `command_id` field is present and non-empty.
/// 3. `action` field is present.
/// 4. `action` is one of `"lock"` or `"unlock"`.
/// 5. `doors` array is present.
///
/// Returns the deserialized [`CommandPayload`] on success.
pub fn validate_command_payload(payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    // Step 1: Parse as generic JSON value.
    let value: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| ValidationError::InvalidJson(e.to_string()))?;

    let obj = value
        .as_object()
        .ok_or_else(|| ValidationError::InvalidJson("payload is not a JSON object".to_string()))?;

    // Step 2: Validate `command_id` — must be present and non-empty string.
    match obj.get("command_id") {
        None => return Err(ValidationError::MissingField("command_id".to_string())),
        Some(v) => {
            let s = v
                .as_str()
                .ok_or_else(|| ValidationError::MissingField("command_id".to_string()))?;
            if s.is_empty() {
                return Err(ValidationError::MissingField("command_id".to_string()));
            }
        }
    }

    // Step 3: Validate `action` — must be present.
    let action = match obj.get("action") {
        None => return Err(ValidationError::MissingField("action".to_string())),
        Some(v) => v
            .as_str()
            .ok_or_else(|| ValidationError::MissingField("action".to_string()))?,
    };

    // Step 4: Validate `action` value — must be "lock" or "unlock".
    if action != "lock" && action != "unlock" {
        return Err(ValidationError::InvalidAction(action.to_string()));
    }

    // Step 5: Validate `doors` — must be present and be an array.
    match obj.get("doors") {
        None => return Err(ValidationError::MissingField("doors".to_string())),
        Some(v) => {
            if !v.is_array() {
                return Err(ValidationError::MissingField("doors".to_string()));
            }
        }
    }

    // All checks passed — deserialize into the typed struct.
    // Individual door element values are not validated here (REQ-6.4).
    serde_json::from_slice(payload).map_err(|e| ValidationError::InvalidJson(e.to_string()))
}

// ─────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ── Bearer token tests ────────────────────────────────

    // TS-04-3: Bearer token validation accepts valid token
    // Validates: [04-REQ-5.1], [04-REQ-5.2]
    #[test]
    fn ts_04_3_bearer_token_valid() {
        let result = validate_bearer_token(Some("Bearer demo-token"), "demo-token");
        assert!(result.is_ok(), "expected Ok(()), got {:?}", result);
    }

    // TS-04-E2: Bearer token validation rejects missing header
    // Validates: [04-REQ-5.E1]
    #[test]
    fn ts_04_e2_bearer_token_missing_header() {
        let result = validate_bearer_token(None, "demo-token");
        assert_eq!(result, Err(AuthError::MissingHeader));
    }

    // TS-04-E3: Bearer token validation rejects wrong token
    // Validates: [04-REQ-5.E2]
    #[test]
    fn ts_04_e3_bearer_token_wrong_token() {
        let result = validate_bearer_token(Some("Bearer wrong-token"), "demo-token");
        assert_eq!(result, Err(AuthError::InvalidToken));
    }

    // TS-04-E4: Bearer token validation rejects malformed header
    // Validates: [04-REQ-5.E2]
    #[test]
    fn ts_04_e4_bearer_token_malformed_header() {
        let result = validate_bearer_token(Some("NotBearer demo-token"), "demo-token");
        assert_eq!(result, Err(AuthError::InvalidToken));
    }

    // ── Payload validation tests ──────────────────────────

    // TS-04-4: Command validation accepts valid lock payload
    // Validates: [04-REQ-6.1], [04-REQ-6.2]
    #[test]
    fn ts_04_4_command_validation_accepts_valid_lock() {
        let payload = br#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}"#;
        let result = validate_command_payload(payload);
        let cmd = result.expect("should return Ok(cmd)");
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, "lock");
        assert_eq!(cmd.doors.len(), 1);
        assert_eq!(cmd.doors[0], "driver");
    }

    // TS-04-5: Command validation accepts unlock action
    // Validates: [04-REQ-6.2]
    #[test]
    fn ts_04_5_command_validation_accepts_unlock() {
        let payload = br#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        let cmd = result.expect("should return Ok(cmd)");
        assert_eq!(cmd.action, "unlock");
    }

    // TS-04-E5: Command validation rejects invalid JSON
    // Validates: [04-REQ-6.E1]
    #[test]
    fn ts_04_e5_command_validation_rejects_invalid_json() {
        let payload = b"not-valid-json{{";
        let result = validate_command_payload(payload);
        assert!(
            matches!(result, Err(ValidationError::InvalidJson(_))),
            "expected Err(InvalidJson), got {:?}",
            result
        );
    }

    // TS-04-E6: Command validation rejects missing command_id
    // Validates: [04-REQ-6.E2]
    #[test]
    fn ts_04_e6_command_validation_rejects_missing_command_id() {
        let payload = br#"{"action":"lock","doors":["driver"]}"#;
        let err = validate_command_payload(payload)
            .expect_err("expected Err for missing command_id");
        assert_eq!(err, ValidationError::MissingField("command_id".to_string()));
    }

    // TS-04-E7: Command validation rejects empty command_id
    // Validates: [04-REQ-6.E2]
    #[test]
    fn ts_04_e7_command_validation_rejects_empty_command_id() {
        let payload = br#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let err = validate_command_payload(payload)
            .expect_err("expected Err for empty command_id");
        assert_eq!(err, ValidationError::MissingField("command_id".to_string()));
    }

    // TS-04-E8: Command validation rejects missing action
    // Validates: [04-REQ-6.E2]
    #[test]
    fn ts_04_e8_command_validation_rejects_missing_action() {
        let payload = br#"{"command_id":"abc","doors":["driver"]}"#;
        let err = validate_command_payload(payload)
            .expect_err("expected Err for missing action");
        assert_eq!(err, ValidationError::MissingField("action".to_string()));
    }

    // TS-04-E9: Command validation rejects invalid action
    // Validates: [04-REQ-6.E3]
    #[test]
    fn ts_04_e9_command_validation_rejects_invalid_action() {
        let payload = br#"{"command_id":"abc","action":"open","doors":["driver"]}"#;
        let err = validate_command_payload(payload)
            .expect_err("expected Err for invalid action");
        assert_eq!(err, ValidationError::InvalidAction("open".to_string()));
    }

    // TS-04-E10: Command validation rejects missing doors
    // Validates: [04-REQ-6.E2]
    #[test]
    fn ts_04_e10_command_validation_rejects_missing_doors() {
        let payload = br#"{"command_id":"abc","action":"lock"}"#;
        let err = validate_command_payload(payload)
            .expect_err("expected Err for missing doors");
        assert_eq!(err, ValidationError::MissingField("doors".to_string()));
    }

    // TS-04-6: Command validation does not validate individual door values
    // Validates: [04-REQ-6.4]
    #[test]
    fn ts_04_6_command_validation_does_not_validate_door_values() {
        let payload =
            br#"{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}"#;
        let result = validate_command_payload(payload);
        let cmd = result.expect("should return Ok(cmd)");
        assert_eq!(cmd.doors.len(), 2);
        assert_eq!(cmd.doors[0], "unknown-door");
        assert_eq!(cmd.doors[1], "another");
    }

    // ── Property tests ────────────────────────────────────

    // TS-04-P2: Command Structural Validity
    // Validates: [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.3], [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3]
    //
    // For any payload: validate_command_payload succeeds IFF the payload is valid
    // JSON with a non-empty command_id, action in {"lock","unlock"}, and a doors
    // array present.
    proptest! {
        #[test]
        fn ts_04_p2_command_structural_validity(
            command_id in proptest::option::of("[a-zA-Z0-9-]{1,32}"),
            action in proptest::option::of(prop_oneof!["lock", "unlock", "open", "close", "bad"]),
            has_doors in proptest::bool::ANY,
        ) {
            let mut obj = serde_json::Map::new();

            let has_valid_command_id = command_id.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
            let has_valid_action = action.as_deref().map(|a| a == "lock" || a == "unlock").unwrap_or(false);

            if let Some(cid) = &command_id {
                obj.insert("command_id".to_string(), serde_json::Value::String(cid.clone()));
            }
            if let Some(act) = &action {
                obj.insert("action".to_string(), serde_json::Value::String(act.to_string()));
            }
            if has_doors {
                obj.insert("doors".to_string(), serde_json::Value::Array(vec!["driver".into()]));
            }

            let payload = serde_json::to_vec(&obj).unwrap();
            let result = validate_command_payload(&payload);

            let should_succeed = has_valid_command_id && has_valid_action && has_doors;

            if should_succeed {
                prop_assert!(result.is_ok(), "expected Ok but got {:?}", result);
            } else {
                prop_assert!(result.is_err(), "expected Err but got Ok");
            }
        }
    }

    // TS-04-P3: Command Passthrough Fidelity
    // Validates: [04-REQ-6.3], [04-REQ-6.4]
    //
    // For any valid command payload, the validator preserves all fields —
    // command_id, action, doors, AND any extra fields present. The original
    // NATS payload bytes are what get written to DATA_BROKER (see main.rs
    // line 161-168), so the validator must not cause data loss during parsing.
    // (Full end-to-end byte-for-byte fidelity is verified by integration
    // test TS-04-10; here we verify that the parsed struct retains all data.)
    proptest! {
        #[test]
        fn ts_04_p3_command_passthrough_fidelity(
            command_id in "[a-zA-Z0-9-]{1,32}",
            action in prop_oneof!["lock", "unlock"],
            extra_key in "x_[a-z]{1,8}",
            extra_value in "[a-zA-Z0-9]{1,16}",
        ) {
            // Build a payload with extra fields beyond the required ones,
            // mirroring real payloads that include source, vin, timestamp, etc.
            let payload_str = format!(
                r#"{{"command_id":"{cmd}","action":"{act}","doors":["driver"],"{key}":"{val}"}}"#,
                cmd = command_id,
                act = action,
                key = extra_key,
                val = extra_value,
            );
            let payload = payload_str.as_bytes();

            let result = validate_command_payload(payload);
            let cmd = result.expect("valid payload must parse successfully");

            // The parsed command_id and action must match the input exactly.
            prop_assert_eq!(cmd.command_id.as_str(), command_id.as_str());
            prop_assert_eq!(cmd.action.as_str(), action);

            // Extra fields must be preserved in the flattened map (REQ-6.4:
            // the validator does not strip unknown fields).
            prop_assert!(
                cmd.extra.contains_key(&extra_key),
                "extra field '{}' must be preserved in parsed payload", extra_key
            );
            prop_assert_eq!(
                cmd.extra.get(&extra_key).and_then(|v| v.as_str()),
                Some(extra_value.as_str()),
                "extra field value must match original"
            );

            // Verify doors are preserved as well.
            prop_assert_eq!(cmd.doors.len(), 1);
            prop_assert_eq!(&cmd.doors[0], "driver");
        }
    }
}
