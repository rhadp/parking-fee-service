//! Unit tests for the `command_validator` module.
//!
//! Tests cover:
//! - TS-04-3: Bearer token validation accepts valid token
//! - TS-04-E2: Bearer token validation rejects missing header
//! - TS-04-E3: Bearer token validation rejects wrong token
//! - TS-04-E4: Bearer token validation rejects malformed header
//! - TS-04-4: Command validation accepts valid payload
//! - TS-04-5: Command validation accepts unlock action
//! - TS-04-E5: Command validation rejects invalid JSON
//! - TS-04-E6: Command validation rejects missing command_id
//! - TS-04-E7: Command validation rejects empty command_id
//! - TS-04-E8: Command validation rejects missing action
//! - TS-04-E9: Command validation rejects invalid action
//! - TS-04-E10: Command validation rejects missing doors
//! - TS-04-6: Command validation does not validate door values
//!
//! Requirements: [04-REQ-5.1], [04-REQ-5.2], [04-REQ-5.E1], [04-REQ-5.E2],
//!               [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.4],
//!               [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3]

use cloud_gateway_client::command_validator::{validate_bearer_token, validate_command_payload};
use cloud_gateway_client::errors::{AuthError, ValidationError};

// ===========================================================================
// Bearer Token Validation Tests
// ===========================================================================

// ---------------------------------------------------------------------------
// TS-04-3: Bearer token validation accepts valid token
// Validates: [04-REQ-5.1], [04-REQ-5.2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_3_bearer_token_valid() {
    // GIVEN headers contain "Authorization" = "Bearer demo-token"
    // GIVEN expected_token = "demo-token"
    let auth_header = Some("Bearer demo-token");
    let expected = "demo-token";

    // WHEN validate_bearer_token(headers, expected_token) is called
    let result = validate_bearer_token(auth_header, expected);

    // THEN result is Ok(())
    assert!(result.is_ok(), "valid bearer token should be accepted");
}

// ---------------------------------------------------------------------------
// TS-04-E2: Bearer token validation rejects missing header
// Validates: [04-REQ-5.E1]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_e2_bearer_token_missing_header() {
    // GIVEN headers do not contain "Authorization"
    // GIVEN expected_token = "demo-token"
    let auth_header: Option<&str> = None;
    let expected = "demo-token";

    // WHEN validate_bearer_token(headers, expected_token) is called
    let result = validate_bearer_token(auth_header, expected);

    // THEN result is Err(AuthError::MissingHeader)
    assert_eq!(
        result.unwrap_err(),
        AuthError::MissingHeader,
        "missing Authorization header should return MissingHeader"
    );
}

// ---------------------------------------------------------------------------
// TS-04-E3: Bearer token validation rejects wrong token
// Validates: [04-REQ-5.E2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_e3_bearer_token_wrong_token() {
    // GIVEN headers contain "Authorization" = "Bearer wrong-token"
    // GIVEN expected_token = "demo-token"
    let auth_header = Some("Bearer wrong-token");
    let expected = "demo-token";

    // WHEN validate_bearer_token(headers, expected_token) is called
    let result = validate_bearer_token(auth_header, expected);

    // THEN result is Err(AuthError::InvalidToken)
    assert_eq!(
        result.unwrap_err(),
        AuthError::InvalidToken,
        "wrong bearer token should return InvalidToken"
    );
}

// ---------------------------------------------------------------------------
// TS-04-E4: Bearer token validation rejects malformed header
// Validates: [04-REQ-5.E2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_e4_bearer_token_malformed_header() {
    // GIVEN headers contain "Authorization" = "NotBearer demo-token"
    // GIVEN expected_token = "demo-token"
    let auth_header = Some("NotBearer demo-token");
    let expected = "demo-token";

    // WHEN validate_bearer_token(headers, expected_token) is called
    let result = validate_bearer_token(auth_header, expected);

    // THEN result is Err(AuthError::InvalidToken)
    assert_eq!(
        result.unwrap_err(),
        AuthError::InvalidToken,
        "malformed Authorization header should return InvalidToken"
    );
}

// ===========================================================================
// Command Payload Validation Tests
// ===========================================================================

// ---------------------------------------------------------------------------
// TS-04-4: Command validation accepts valid payload
// Validates: [04-REQ-6.1], [04-REQ-6.2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_4_command_valid_lock() {
    // GIVEN payload with all required fields and extra fields
    let payload = br#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}"#;

    // WHEN validate_command_payload(payload) is called
    let result = validate_command_payload(payload);

    // THEN result is Ok(cmd)
    let cmd = result.expect("valid payload should be accepted");
    // AND cmd.command_id == "abc-123"
    assert_eq!(cmd.command_id, "abc-123");
    // AND cmd.action == "lock"
    assert_eq!(cmd.action, "lock");
    // AND cmd.doors == ["driver"]
    assert_eq!(cmd.doors, vec!["driver"]);
}

// ---------------------------------------------------------------------------
// TS-04-5: Command validation accepts unlock action
// Validates: [04-REQ-6.2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_5_command_valid_unlock() {
    // GIVEN payload with action "unlock"
    let payload = br#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;

    // WHEN validate_command_payload(payload) is called
    let result = validate_command_payload(payload);

    // THEN result is Ok(cmd)
    let cmd = result.expect("unlock payload should be accepted");
    // AND cmd.action == "unlock"
    assert_eq!(cmd.action, "unlock");
}

// ---------------------------------------------------------------------------
// TS-04-E5: Command validation rejects invalid JSON
// Validates: [04-REQ-6.E1]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_e5_command_invalid_json() {
    // GIVEN payload is not valid JSON
    let payload = b"not-valid-json{{";

    // WHEN validate_command_payload(payload) is called
    let result = validate_command_payload(payload);

    // THEN result is Err(ValidationError::InvalidJson(_))
    assert!(
        matches!(result, Err(ValidationError::InvalidJson(_))),
        "invalid JSON should return InvalidJson, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// TS-04-E6: Command validation rejects missing command_id
// Validates: [04-REQ-6.E2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_e6_command_missing_command_id() {
    // GIVEN payload without command_id
    let payload = br#"{"action":"lock","doors":["driver"]}"#;

    // WHEN validate_command_payload(payload) is called
    let result = validate_command_payload(payload);

    // THEN result is Err(ValidationError::MissingField("command_id"))
    match result {
        Err(ValidationError::MissingField(field)) => {
            assert_eq!(field, "command_id", "should report missing command_id");
        }
        other => panic!(
            "expected MissingField(\"command_id\"), got {:?}",
            other
        ),
    }
}

// ---------------------------------------------------------------------------
// TS-04-E7: Command validation rejects empty command_id
// Validates: [04-REQ-6.E2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_e7_command_empty_command_id() {
    // GIVEN payload with empty command_id
    let payload = br#"{"command_id":"","action":"lock","doors":["driver"]}"#;

    // WHEN validate_command_payload(payload) is called
    let result = validate_command_payload(payload);

    // THEN result is Err(ValidationError::MissingField("command_id"))
    match result {
        Err(ValidationError::MissingField(field)) => {
            assert_eq!(field, "command_id", "should report missing command_id");
        }
        other => panic!(
            "expected MissingField(\"command_id\"), got {:?}",
            other
        ),
    }
}

// ---------------------------------------------------------------------------
// TS-04-E8: Command validation rejects missing action
// Validates: [04-REQ-6.E2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_e8_command_missing_action() {
    // GIVEN payload without action
    let payload = br#"{"command_id":"abc","doors":["driver"]}"#;

    // WHEN validate_command_payload(payload) is called
    let result = validate_command_payload(payload);

    // THEN result is Err(ValidationError::MissingField("action"))
    match result {
        Err(ValidationError::MissingField(field)) => {
            assert_eq!(field, "action", "should report missing action");
        }
        other => panic!("expected MissingField(\"action\"), got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// TS-04-E9: Command validation rejects invalid action
// Validates: [04-REQ-6.E3]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_e9_command_invalid_action() {
    // GIVEN payload with invalid action "open"
    let payload = br#"{"command_id":"abc","action":"open","doors":["driver"]}"#;

    // WHEN validate_command_payload(payload) is called
    let result = validate_command_payload(payload);

    // THEN result is Err(ValidationError::InvalidAction("open"))
    match result {
        Err(ValidationError::InvalidAction(action)) => {
            assert_eq!(action, "open", "should report invalid action 'open'");
        }
        other => panic!("expected InvalidAction(\"open\"), got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// TS-04-E10: Command validation rejects missing doors
// Validates: [04-REQ-6.E2]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_e10_command_missing_doors() {
    // GIVEN payload without doors
    let payload = br#"{"command_id":"abc","action":"lock"}"#;

    // WHEN validate_command_payload(payload) is called
    let result = validate_command_payload(payload);

    // THEN result is Err(ValidationError::MissingField("doors"))
    match result {
        Err(ValidationError::MissingField(field)) => {
            assert_eq!(field, "doors", "should report missing doors");
        }
        other => panic!("expected MissingField(\"doors\"), got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// TS-04-6: Command validation does not validate door values
// Validates: [04-REQ-6.4]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_6_command_does_not_validate_door_values() {
    // GIVEN payload with unknown door values
    let payload = br#"{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}"#;

    // WHEN validate_command_payload(payload) is called
    let result = validate_command_payload(payload);

    // THEN result is Ok(cmd)
    let cmd = result.expect("unknown door values should be accepted");
    // AND cmd.doors == ["unknown-door", "another"]
    assert_eq!(cmd.doors, vec!["unknown-door", "another"]);
}
