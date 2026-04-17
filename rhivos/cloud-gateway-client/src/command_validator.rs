#![allow(dead_code)]

use std::collections::HashMap;

use crate::errors::{AuthError, ValidationError};
use crate::models::CommandPayload;

/// A simple string-keyed header map used for bearer-token validation.
///
/// Unit tests construct headers with this type.  The production wiring
/// (task group 5) will extract values from `async_nats::HeaderMap` and
/// pass them here, keeping this module I/O-free and easily testable.
pub type HeaderMap = HashMap<String, String>;

/// Validates the `Authorization` bearer token in a NATS message header map.
///
/// Expects the header value to be exactly `Bearer <expected>`.
/// Returns `Ok(())` on success, or an `AuthError` variant on failure.
///
/// Validates [04-REQ-5.1], [04-REQ-5.2], [04-REQ-5.E1], [04-REQ-5.E2]
pub fn validate_bearer_token(headers: &HeaderMap, expected: &str) -> Result<(), AuthError> {
    let auth_value = headers.get("Authorization").ok_or(AuthError::MissingHeader)?;
    let expected_value = format!("Bearer {}", expected);
    if auth_value == &expected_value {
        Ok(())
    } else {
        Err(AuthError::InvalidToken)
    }
}

/// Validates the structure of a command payload received from NATS.
///
/// Checks that the payload is valid JSON containing:
/// - `command_id`: non-empty string
/// - `action`: one of `"lock"` or `"unlock"`
/// - `doors`: array (individual element types are NOT validated — REQ-6.4)
///
/// Returns the parsed `CommandPayload` on success, or a `ValidationError`.
///
/// Validates [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.4],
///           [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3]
pub fn validate_command_payload(payload: &[u8]) -> Result<CommandPayload, ValidationError> {
    // Parse as generic JSON value first to give structured field-level errors.
    let value: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| ValidationError::InvalidJson(e.to_string()))?;

    let obj = value
        .as_object()
        .ok_or_else(|| ValidationError::InvalidJson("expected a JSON object".to_string()))?;

    // Validate command_id: must be a non-empty string.
    let command_id = match obj.get("command_id") {
        Some(serde_json::Value::String(s)) if !s.is_empty() => s.clone(),
        Some(serde_json::Value::String(_)) => {
            // Empty string — treat as missing.
            return Err(ValidationError::MissingField("command_id".to_string()));
        }
        _ => return Err(ValidationError::MissingField("command_id".to_string())),
    };

    // Validate action: must be present as a string.
    let action = match obj.get("action") {
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => return Err(ValidationError::MissingField("action".to_string())),
    };

    // Validate action value: must be "lock" or "unlock".
    if action != "lock" && action != "unlock" {
        return Err(ValidationError::InvalidAction(action));
    }

    // Validate doors: must be present as a JSON array.
    let doors = match obj.get("doors") {
        Some(serde_json::Value::Array(arr)) => arr.clone(),
        _ => return Err(ValidationError::MissingField("doors".to_string())),
    };

    // Collect all extra fields (everything beyond command_id, action, doors).
    let mut extra = serde_json::Map::new();
    for (k, v) in obj {
        if k != "command_id" && k != "action" && k != "doors" {
            extra.insert(k.clone(), v.clone());
        }
    }

    Ok(CommandPayload {
        command_id,
        action,
        doors,
        extra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // ------------------------------------------------------------------
    // Bearer-token validation — TS-04-3, TS-04-E2, TS-04-E3, TS-04-E4
    // ------------------------------------------------------------------

    /// TS-04-3: Valid token accepted.
    ///
    /// Validates [04-REQ-5.1], [04-REQ-5.2]
    #[test]
    fn test_valid_bearer_token() {
        let hdrs = headers(&[("Authorization", "Bearer demo-token")]);
        let result = validate_bearer_token(&hdrs, "demo-token");
        assert!(result.is_ok(), "expected Ok(()), got {:?}", result);
    }

    /// TS-04-E2: Missing Authorization header returns MissingHeader.
    ///
    /// Validates [04-REQ-5.E1]
    #[test]
    fn test_missing_authorization_header() {
        let hdrs = headers(&[]);
        let result = validate_bearer_token(&hdrs, "demo-token");
        assert!(
            matches!(result, Err(AuthError::MissingHeader)),
            "expected Err(MissingHeader), got {:?}",
            result
        );
    }

    /// TS-04-E3: Wrong token value returns InvalidToken.
    ///
    /// Validates [04-REQ-5.E2]
    #[test]
    fn test_wrong_bearer_token() {
        let hdrs = headers(&[("Authorization", "Bearer wrong-token")]);
        let result = validate_bearer_token(&hdrs, "demo-token");
        assert!(
            matches!(result, Err(AuthError::InvalidToken)),
            "expected Err(InvalidToken), got {:?}",
            result
        );
    }

    /// TS-04-E4: Malformed Authorization header (not `Bearer …`) returns InvalidToken.
    ///
    /// Validates [04-REQ-5.E2]
    #[test]
    fn test_malformed_bearer_header() {
        let hdrs = headers(&[("Authorization", "NotBearer demo-token")]);
        let result = validate_bearer_token(&hdrs, "demo-token");
        assert!(
            matches!(result, Err(AuthError::InvalidToken)),
            "expected Err(InvalidToken), got {:?}",
            result
        );
    }

    // ------------------------------------------------------------------
    // Command payload validation
    // TS-04-4, TS-04-5, TS-04-E5 – TS-04-E10, TS-04-6
    // ------------------------------------------------------------------

    /// TS-04-4: Valid lock command with extra fields is accepted.
    ///
    /// Validates [04-REQ-6.1], [04-REQ-6.2]
    #[test]
    fn test_valid_lock_payload() {
        let payload = br#"{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}"#;
        let result = validate_command_payload(payload);
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        let cmd = result.unwrap();
        assert_eq!(cmd.command_id, "abc-123");
        assert_eq!(cmd.action, "lock");
        assert_eq!(cmd.doors, vec![serde_json::json!("driver")]);
    }

    /// TS-04-5: Valid unlock command is accepted.
    ///
    /// Validates [04-REQ-6.2]
    #[test]
    fn test_valid_unlock_payload() {
        let payload = br#"{"command_id":"def-456","action":"unlock","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        let cmd = result.unwrap();
        assert_eq!(cmd.action, "unlock");
    }

    /// TS-04-E5: Non-JSON payload returns InvalidJson.
    ///
    /// Validates [04-REQ-6.E1]
    #[test]
    fn test_invalid_json_payload() {
        let payload = b"not-valid-json{{";
        let result = validate_command_payload(payload);
        assert!(
            matches!(result, Err(ValidationError::InvalidJson(_))),
            "expected Err(InvalidJson), got {:?}",
            result
        );
    }

    /// TS-04-E6: Missing `command_id` field returns MissingField("command_id").
    ///
    /// Validates [04-REQ-6.E2]
    #[test]
    fn test_missing_command_id() {
        let payload = br#"{"action":"lock","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        assert!(
            matches!(&result, Err(ValidationError::MissingField(f)) if f == "command_id"),
            "expected Err(MissingField(\"command_id\")), got {:?}",
            result
        );
    }

    /// TS-04-E7: Empty `command_id` returns MissingField("command_id").
    ///
    /// Validates [04-REQ-6.E2]
    #[test]
    fn test_empty_command_id() {
        let payload = br#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        assert!(
            matches!(&result, Err(ValidationError::MissingField(f)) if f == "command_id"),
            "expected Err(MissingField(\"command_id\")), got {:?}",
            result
        );
    }

    /// TS-04-E8: Missing `action` field returns MissingField("action").
    ///
    /// Validates [04-REQ-6.E2]
    #[test]
    fn test_missing_action() {
        let payload = br#"{"command_id":"abc","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        assert!(
            matches!(&result, Err(ValidationError::MissingField(f)) if f == "action"),
            "expected Err(MissingField(\"action\")), got {:?}",
            result
        );
    }

    /// TS-04-E9: Unsupported `action` value returns InvalidAction with the bad value.
    ///
    /// Validates [04-REQ-6.E3]
    #[test]
    fn test_invalid_action_value() {
        let payload = br#"{"command_id":"abc","action":"open","doors":["driver"]}"#;
        let result = validate_command_payload(payload);
        assert!(
            matches!(&result, Err(ValidationError::InvalidAction(a)) if a == "open"),
            "expected Err(InvalidAction(\"open\")), got {:?}",
            result
        );
    }

    /// TS-04-E10: Missing `doors` field returns MissingField("doors").
    ///
    /// Validates [04-REQ-6.E2]
    #[test]
    fn test_missing_doors() {
        let payload = br#"{"command_id":"abc","action":"lock"}"#;
        let result = validate_command_payload(payload);
        assert!(
            matches!(&result, Err(ValidationError::MissingField(f)) if f == "doors"),
            "expected Err(MissingField(\"doors\")), got {:?}",
            result
        );
    }

    /// TS-04-6: Individual door values are not validated; arbitrary values accepted.
    ///
    /// Validates [04-REQ-6.4]
    #[test]
    fn test_arbitrary_door_values_accepted() {
        let payload =
            br#"{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}"#;
        let result = validate_command_payload(payload);
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        let cmd = result.unwrap();
        assert_eq!(
            cmd.doors,
            vec![
                serde_json::json!("unknown-door"),
                serde_json::json!("another")
            ]
        );
    }

    // ------------------------------------------------------------------
    // Property test: Command Structural Validity — TS-04-P2
    // ------------------------------------------------------------------

    /// TS-04-P2: For any payload, validation returns Ok iff the payload is valid JSON
    /// with a non-empty command_id, action in {"lock","unlock"}, and a doors array.
    ///
    /// Validates [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3]
    #[test]
    fn test_property_command_structural_validity() {
        struct Case {
            payload: &'static [u8],
            should_succeed: bool,
        }

        let cases = [
            // --- valid ---
            Case {
                payload: br#"{"command_id":"x","action":"lock","doors":[]}"#,
                should_succeed: true,
            },
            Case {
                payload: br#"{"command_id":"x","action":"unlock","doors":[]}"#,
                should_succeed: true,
            },
            Case {
                payload: br#"{"command_id":"x","action":"lock","doors":["a","b"]}"#,
                should_succeed: true,
            },
            Case {
                payload: br#"{"command_id":"x","action":"lock","doors":["a"],"extra":"ignored"}"#,
                should_succeed: true,
            },
            // --- invalid: not JSON ---
            Case {
                payload: b"not json at all",
                should_succeed: false,
            },
            // --- invalid: empty command_id ---
            Case {
                payload: br#"{"command_id":"","action":"lock","doors":[]}"#,
                should_succeed: false,
            },
            // --- invalid: missing command_id ---
            Case {
                payload: br#"{"action":"lock","doors":[]}"#,
                should_succeed: false,
            },
            // --- invalid: unsupported action ---
            Case {
                payload: br#"{"command_id":"x","action":"open","doors":[]}"#,
                should_succeed: false,
            },
            // --- invalid: missing action ---
            Case {
                payload: br#"{"command_id":"x","doors":[]}"#,
                should_succeed: false,
            },
            // --- invalid: missing doors ---
            Case {
                payload: br#"{"command_id":"x","action":"lock"}"#,
                should_succeed: false,
            },
        ];

        for case in &cases {
            let result = validate_command_payload(case.payload);
            if case.should_succeed {
                assert!(
                    result.is_ok(),
                    "Expected Ok for {:?}, got {:?}",
                    std::str::from_utf8(case.payload),
                    result.err()
                );
            } else {
                assert!(
                    result.is_err(),
                    "Expected Err for {:?}, got Ok",
                    std::str::from_utf8(case.payload)
                );
            }
        }
    }
}
