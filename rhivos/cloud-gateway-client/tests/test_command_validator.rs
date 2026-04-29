//! Unit tests for the command_validator module.
//!
//! Test Spec: TS-04-3, TS-04-4, TS-04-5, TS-04-6,
//!            TS-04-E2, TS-04-E3, TS-04-E4, TS-04-E5,
//!            TS-04-E6, TS-04-E7, TS-04-E8, TS-04-E9, TS-04-E10
//! Requirements: 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.E1, 04-REQ-5.E2,
//!               04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.4,
//!               04-REQ-6.E1, 04-REQ-6.E2, 04-REQ-6.E3

use std::collections::HashMap;

use cloud_gateway_client::command_validator::{validate_bearer_token, validate_command_payload};
use cloud_gateway_client::errors::{AuthError, ValidationError};

// ---------------------------------------------------------------------------
// Bearer token validation tests
// ---------------------------------------------------------------------------

/// TS-04-3: Bearer token validation accepts valid token.
///
/// Requirements: 04-REQ-5.1, 04-REQ-5.2
/// WHEN the Authorization header matches Bearer <configured_token>,
/// the system SHALL proceed with command validation.
#[test]
fn test_bearer_token_valid() {
    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        "Bearer demo-token".to_string(),
    );

    let result = validate_bearer_token(&headers, "demo-token");
    assert!(result.is_ok());
}

/// TS-04-E2: Bearer token validation rejects missing header.
///
/// Requirement: 04-REQ-5.E1
/// WHEN the Authorization header is missing, the system SHALL discard
/// the message.
#[test]
fn test_bearer_token_missing_header() {
    let headers = HashMap::new();

    let result = validate_bearer_token(&headers, "demo-token");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), AuthError::MissingHeader);
}

/// TS-04-E3: Bearer token validation rejects wrong token.
///
/// Requirement: 04-REQ-5.E2
/// WHEN the token does not match the configured value, the system SHALL
/// discard the message.
#[test]
fn test_bearer_token_wrong_token() {
    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        "Bearer wrong-token".to_string(),
    );

    let result = validate_bearer_token(&headers, "demo-token");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), AuthError::InvalidToken);
}

/// TS-04-E4: Bearer token validation rejects malformed header.
///
/// Requirement: 04-REQ-5.E2
/// WHEN the Authorization header has the wrong prefix (not "Bearer "),
/// the system SHALL discard the message.
#[test]
fn test_bearer_token_malformed_header() {
    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        "NotBearer demo-token".to_string(),
    );

    let result = validate_bearer_token(&headers, "demo-token");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), AuthError::InvalidToken);
}

// ---------------------------------------------------------------------------
// Command payload validation tests
// ---------------------------------------------------------------------------

/// TS-04-4: Command validation accepts valid lock payload.
///
/// Requirements: 04-REQ-6.1, 04-REQ-6.2
/// WHEN the payload is valid JSON with required fields, the system SHALL
/// proceed.
#[test]
fn test_command_valid_lock_payload() {
    let payload = br#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}"#;

    let result = validate_command_payload(payload);

    assert!(result.is_ok());
    let cmd = result.unwrap();
    assert_eq!(cmd.command_id, "abc-123");
    assert_eq!(cmd.action, "lock");
    assert_eq!(cmd.doors, vec!["driver"]);
}

/// TS-04-5: Command validation accepts unlock action.
///
/// Requirement: 04-REQ-6.2
/// WHEN the action is "unlock", the system SHALL accept the command.
#[test]
fn test_command_valid_unlock_payload() {
    let payload = br#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;

    let result = validate_command_payload(payload);

    assert!(result.is_ok());
    let cmd = result.unwrap();
    assert_eq!(cmd.action, "unlock");
}

/// TS-04-6: Command validation does not validate door values.
///
/// Requirement: 04-REQ-6.4
/// The system SHALL NOT validate individual door values in the doors array.
#[test]
fn test_command_door_values_not_validated() {
    let payload = br#"{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}"#;

    let result = validate_command_payload(payload);

    assert!(result.is_ok());
    let cmd = result.unwrap();
    assert_eq!(cmd.doors, vec!["unknown-door", "another"]);
}

/// TS-04-E5: Command validation rejects invalid JSON.
///
/// Requirement: 04-REQ-6.E1
/// WHEN the payload is not valid JSON, the system SHALL discard the message.
#[test]
fn test_command_invalid_json() {
    let payload = b"not-valid-json{{";

    let result = validate_command_payload(payload);

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), ValidationError::InvalidJson(_)),
        "Expected InvalidJson error"
    );
}

/// TS-04-E6: Command validation rejects missing command_id.
///
/// Requirement: 04-REQ-6.E2
/// WHEN command_id is missing, the system SHALL discard the message.
#[test]
fn test_command_missing_command_id() {
    let payload = br#"{"action":"lock","doors":["driver"]}"#;

    let result = validate_command_payload(payload);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ValidationError::MissingField("command_id".to_string())
    );
}

/// TS-04-E7: Command validation rejects empty command_id.
///
/// Requirement: 04-REQ-6.E2
/// WHEN command_id is an empty string, the system SHALL discard the message.
#[test]
fn test_command_empty_command_id() {
    let payload = br#"{"command_id":"","action":"lock","doors":["driver"]}"#;

    let result = validate_command_payload(payload);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ValidationError::MissingField("command_id".to_string())
    );
}

/// TS-04-E8: Command validation rejects missing action.
///
/// Requirement: 04-REQ-6.E2
/// WHEN the action field is missing, the system SHALL discard the message.
#[test]
fn test_command_missing_action() {
    let payload = br#"{"command_id":"abc","doors":["driver"]}"#;

    let result = validate_command_payload(payload);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ValidationError::MissingField("action".to_string())
    );
}

/// TS-04-E9: Command validation rejects invalid action.
///
/// Requirement: 04-REQ-6.E3
/// WHEN the action is not "lock" or "unlock", the system SHALL discard
/// the message.
#[test]
fn test_command_invalid_action() {
    let payload = br#"{"command_id":"abc","action":"open","doors":["driver"]}"#;

    let result = validate_command_payload(payload);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ValidationError::InvalidAction("open".to_string())
    );
}

/// TS-04-E10: Command validation rejects missing doors.
///
/// Requirement: 04-REQ-6.E2
/// WHEN the doors field is missing, the system SHALL discard the message.
#[test]
fn test_command_missing_doors() {
    let payload = br#"{"command_id":"abc","action":"lock"}"#;

    let result = validate_command_payload(payload);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ValidationError::MissingField("doors".to_string())
    );
}
