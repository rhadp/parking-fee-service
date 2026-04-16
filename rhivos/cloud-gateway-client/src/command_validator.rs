use crate::errors::{AuthError, ValidationError};
use crate::models::CommandPayload;
use async_nats::HeaderMap;

/// Validate the bearer token in the NATS message headers.
///
/// Expects the `Authorization` header to be present and to match
/// `Bearer <expected_token>` exactly (case-sensitive prefix per RFC 7617
/// common practice; see docs/errata for rationale on case-sensitivity choice).
///
/// Returns `Ok(())` if the token matches, or an appropriate `AuthError`.
#[allow(unused_variables)]
pub fn validate_bearer_token(headers: &HeaderMap, expected: &str) -> Result<(), AuthError> {
    todo!("validate_bearer_token not yet implemented")
}

/// Validate a raw command payload (JSON bytes) from NATS.
///
/// Checks:
/// 1. The bytes are valid JSON.
/// 2. `command_id` is present and non-empty.
/// 3. `action` is present and is one of `"lock"` or `"unlock"`.
/// 4. `doors` is present and is a JSON array.
///
/// Individual `doors` values are NOT validated (REQ-6.4).
#[allow(unused_variables)]
pub fn validate_command_payload(payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    todo!("validate_command_payload not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Bearer token tests ──────────────────────────────────────────────────

    fn make_headers(auth: &str) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert("Authorization", auth);
        map
    }

    // TS-04-3: Bearer token validation accepts valid token
    #[test]
    fn ts_04_3_bearer_token_valid() {
        let headers = make_headers("Bearer demo-token");
        let result = validate_bearer_token(&headers, "demo-token");
        assert!(result.is_ok(), "expected Ok for valid token, got {:?}", result);
    }

    // TS-04-E2: Bearer token validation rejects missing header
    #[test]
    fn ts_04_e2_bearer_token_missing_header() {
        let headers = HeaderMap::new();
        let result = validate_bearer_token(&headers, "demo-token");
        assert_eq!(
            result.unwrap_err(),
            AuthError::MissingHeader,
            "expected MissingHeader when Authorization is absent"
        );
    }

    // TS-04-E3: Bearer token validation rejects wrong token
    #[test]
    fn ts_04_e3_bearer_token_wrong_token() {
        let headers = make_headers("Bearer wrong-token");
        let result = validate_bearer_token(&headers, "demo-token");
        assert_eq!(
            result.unwrap_err(),
            AuthError::InvalidToken,
            "expected InvalidToken when token does not match"
        );
    }

    // TS-04-E4: Bearer token validation rejects malformed header (no "Bearer " prefix)
    #[test]
    fn ts_04_e4_bearer_token_malformed_header() {
        let headers = make_headers("NotBearer demo-token");
        let result = validate_bearer_token(&headers, "demo-token");
        assert_eq!(
            result.unwrap_err(),
            AuthError::InvalidToken,
            "expected InvalidToken when Authorization prefix is not 'Bearer '"
        );
    }

    // ── Command payload validation tests ────────────────────────────────────

    // TS-04-4: Command validation accepts valid lock payload
    #[test]
    fn ts_04_4_command_valid_lock_payload() {
        let payload = br#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}"#;
        let result = validate_command_payload(payload);
        let cmd = result.expect("expected Ok for valid lock command");
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, "lock");
        assert_eq!(
            cmd.doors,
            vec![serde_json::Value::String("driver".to_string())]
        );
    }

    // TS-04-5: Command validation accepts unlock action
    #[test]
    fn ts_04_5_command_valid_unlock_payload() {
        let payload = br#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        let cmd = result.expect("expected Ok for valid unlock command");
        assert_eq!(cmd.action, "unlock");
    }

    // TS-04-E5: Command validation rejects invalid JSON
    #[test]
    fn ts_04_e5_command_invalid_json() {
        let payload = b"not-valid-json{{";
        let result = validate_command_payload(payload);
        assert!(
            matches!(result, Err(ValidationError::InvalidJson(_))),
            "expected InvalidJson error, got {:?}",
            result
        );
    }

    // TS-04-E6: Command validation rejects missing command_id
    #[test]
    fn ts_04_e6_command_missing_command_id() {
        let payload = br#"{"action":"lock","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("command_id".to_string()),
            "expected MissingField(command_id)"
        );
    }

    // TS-04-E7: Command validation rejects empty command_id
    #[test]
    fn ts_04_e7_command_empty_command_id() {
        let payload = br#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("command_id".to_string()),
            "expected MissingField(command_id) for empty command_id"
        );
    }

    // TS-04-E8: Command validation rejects missing action
    #[test]
    fn ts_04_e8_command_missing_action() {
        let payload = br#"{"command_id":"abc","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("action".to_string()),
            "expected MissingField(action)"
        );
    }

    // TS-04-E9: Command validation rejects invalid action value
    #[test]
    fn ts_04_e9_command_invalid_action() {
        let payload = br#"{"command_id":"abc","action":"open","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        assert_eq!(
            result.unwrap_err(),
            ValidationError::InvalidAction("open".to_string()),
            "expected InvalidAction(open)"
        );
    }

    // TS-04-E10: Command validation rejects missing doors
    #[test]
    fn ts_04_e10_command_missing_doors() {
        let payload = br#"{"command_id":"abc","action":"lock"}"#;
        let result = validate_command_payload(payload);
        assert_eq!(
            result.unwrap_err(),
            ValidationError::MissingField("doors".to_string()),
            "expected MissingField(doors)"
        );
    }

    // TS-04-6: Command validation does NOT validate individual door values
    #[test]
    fn ts_04_6_command_does_not_validate_door_values() {
        let payload = br#"{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}"#;
        let result = validate_command_payload(payload);
        let cmd = result.expect("expected Ok when door values are arbitrary strings");
        assert_eq!(
            cmd.doors,
            vec![
                serde_json::Value::String("unknown-door".to_string()),
                serde_json::Value::String("another".to_string()),
            ]
        );
    }

    // ── Property tests ──────────────────────────────────────────────────────

    // TS-04-P2: Command Structural Validity property
    // For any payload, validate_command_payload succeeds iff the payload has:
    // valid JSON, non-empty command_id, action in {lock,unlock}, doors array.
    proptest::proptest! {
        #[test]
        fn ts_04_p2_command_structural_validity(s in proptest::string::string_regex("[a-z0-9-]{1,10}").unwrap()) {
            // Valid command — expect Ok
            let payload = format!(
                r#"{{"command_id":"{}","action":"lock","doors":[]}}"#,
                s
            );
            let result = validate_command_payload(payload.as_bytes());
            proptest::prop_assert!(
                result.is_ok(),
                "expected Ok for valid payload with command_id={}: {:?}",
                s, result
            );
        }
    }

    proptest::proptest! {
        #[test]
        fn ts_04_p2_invalid_action_fails(s in proptest::string::string_regex("[A-Z]{1,10}").unwrap()) {
            // Invalid action (uppercase, not "lock" or "unlock") — expect Err
            let payload = format!(
                r#"{{"command_id":"abc","action":"{}","doors":[]}}"#,
                s
            );
            let result = validate_command_payload(payload.as_bytes());
            proptest::prop_assert!(
                matches!(result, Err(ValidationError::InvalidAction(_))),
                "expected InvalidAction for action={}: {:?}",
                s, result
            );
        }
    }

    // TS-04-P3: Command Passthrough Fidelity
    // The bytes written to DATA_BROKER equal the original payload — this is
    // verified here at the unit level by checking that validate_command_payload
    // preserves the raw fields without modification.
    #[test]
    fn ts_04_p3_command_passthrough_fidelity() {
        let raw = br#"{"command_id":"fidelity-cmd","action":"unlock","doors":["all"],"extra":"preserved"}"#;
        let cmd = validate_command_payload(raw).expect("valid payload should parse");
        // Re-serialize via serde_json to verify fields are preserved
        let reserialized = serde_json::to_value(&cmd.extra).unwrap();
        assert!(
            reserialized.get("extra").is_some(),
            "extra fields in the payload must be preserved in CommandPayload.extra"
        );
    }
}
