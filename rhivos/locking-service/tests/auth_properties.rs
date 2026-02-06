//! Property-based tests for authentication validation.
//!
//! These tests verify Property 1 from the design document:
//! "For any Lock or Unlock command with an invalid or missing Auth_Token,
//! the LOCKING_SERVICE SHALL reject the command and return an authentication
//! error without executing any lock operation."

use locking_service::auth::validate_auth_token;
use locking_service::error::LockingError;
use proptest::prelude::*;

/// Valid tokens used for testing - matches ServiceConfig default
const VALID_TOKENS: &[&str] = &["demo-token"];

/// Strategy to generate arbitrary strings that are NOT valid tokens
fn invalid_token_strategy() -> impl Strategy<Value = String> {
    // Generate random strings and filter out valid tokens
    prop::string::string_regex("[a-zA-Z0-9_-]{0,50}")
        .unwrap()
        .prop_filter("must not be a valid token", |s| {
            !VALID_TOKENS.contains(&s.as_str())
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: locking-service, Property 1: Invalid Auth Token Rejection
    ///
    /// For any invalid auth token, validation SHALL fail with AuthError.
    /// Validates: Requirements 1.4, 2.4
    #[test]
    fn property_invalid_token_rejected(token in invalid_token_strategy()) {
        let valid_tokens: Vec<String> = VALID_TOKENS.iter().map(|s| s.to_string()).collect();

        let result = validate_auth_token(&token, &valid_tokens);

        // Must be an error
        prop_assert!(result.is_err(), "Invalid token '{}' should be rejected", token);

        // Must be an AuthError specifically
        match result {
            Err(LockingError::AuthError(_)) => {
                // Expected - authentication error
            }
            Err(other) => {
                prop_assert!(false, "Expected AuthError, got {:?}", other);
            }
            Ok(()) => {
                prop_assert!(false, "Token '{}' should have been rejected", token);
            }
        }
    }

    /// Feature: locking-service, Property 1: Invalid Auth Token Rejection (gRPC status)
    ///
    /// For any invalid auth token, the resulting gRPC status SHALL be UNAUTHENTICATED.
    /// Validates: Requirements 1.4, 2.4
    #[test]
    fn property_invalid_token_returns_unauthenticated_status(token in invalid_token_strategy()) {
        let valid_tokens: Vec<String> = VALID_TOKENS.iter().map(|s| s.to_string()).collect();

        let result = validate_auth_token(&token, &valid_tokens);

        if let Err(err) = result {
            let status: tonic::Status = err.into();
            prop_assert_eq!(
                status.code(),
                tonic::Code::Unauthenticated,
                "Invalid token should result in UNAUTHENTICATED status"
            );
        } else {
            prop_assert!(false, "Token '{}' should have been rejected", token);
        }
    }

    /// Feature: locking-service, Property 1: Valid Token Acceptance
    ///
    /// For any valid auth token, validation SHALL succeed.
    /// This is the complement to Property 1.
    #[test]
    fn property_valid_token_accepted(index in 0usize..VALID_TOKENS.len()) {
        let valid_tokens: Vec<String> = VALID_TOKENS.iter().map(|s| s.to_string()).collect();
        let token = VALID_TOKENS[index];

        let result = validate_auth_token(token, &valid_tokens);

        prop_assert!(result.is_ok(), "Valid token '{}' should be accepted", token);
    }
}

/// Edge case: Empty token should be rejected as "missing"
#[test]
fn empty_token_rejected_as_missing() {
    let valid_tokens: Vec<String> = VALID_TOKENS.iter().map(|s| s.to_string()).collect();

    let result = validate_auth_token("", &valid_tokens);

    assert!(result.is_err());
    match &result {
        Err(LockingError::AuthError(msg)) => {
            assert!(msg.contains("missing"), "Error should mention 'missing'");
        }
        _ => panic!("Expected AuthError"),
    }

    // Verify gRPC status
    let status: tonic::Status = result.unwrap_err().into();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
}
