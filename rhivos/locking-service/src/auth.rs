//! Authentication validation for LOCKING_SERVICE.
//!
//! This module provides basic authentication token validation for the demo system.
//! Note: This is demo-grade authentication, not suitable for production use.

use crate::error::LockingError;

/// Validates an authentication token against a list of valid tokens.
///
/// # Arguments
///
/// * `token` - The token to validate
/// * `valid_tokens` - List of valid tokens to check against
///
/// # Returns
///
/// * `Ok(())` if the token is valid
/// * `Err(LockingError::AuthError)` if the token is invalid or empty
///
/// # Example
///
/// ```
/// use locking_service::auth::validate_auth_token;
///
/// let valid_tokens = vec!["demo-token".to_string()];
/// assert!(validate_auth_token("demo-token", &valid_tokens).is_ok());
/// assert!(validate_auth_token("invalid", &valid_tokens).is_err());
/// ```
pub fn validate_auth_token(token: &str, valid_tokens: &[String]) -> Result<(), LockingError> {
    if token.is_empty() {
        return Err(LockingError::AuthError("missing auth token".to_string()));
    }

    if valid_tokens.iter().any(|valid| valid == token) {
        Ok(())
    } else {
        Err(LockingError::AuthError(format!(
            "invalid auth token: {}",
            token
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_token() {
        let valid_tokens = vec!["demo-token".to_string(), "test-token".to_string()];
        assert!(validate_auth_token("demo-token", &valid_tokens).is_ok());
        assert!(validate_auth_token("test-token", &valid_tokens).is_ok());
    }

    #[test]
    fn test_invalid_token() {
        let valid_tokens = vec!["demo-token".to_string()];
        let result = validate_auth_token("wrong-token", &valid_tokens);
        assert!(result.is_err());
        match result {
            Err(LockingError::AuthError(msg)) => {
                assert!(msg.contains("invalid"));
            }
            _ => panic!("Expected AuthError"),
        }
    }

    #[test]
    fn test_empty_token() {
        let valid_tokens = vec!["demo-token".to_string()];
        let result = validate_auth_token("", &valid_tokens);
        assert!(result.is_err());
        match result {
            Err(LockingError::AuthError(msg)) => {
                assert!(msg.contains("missing"));
            }
            _ => panic!("Expected AuthError"),
        }
    }

    #[test]
    fn test_empty_valid_tokens_list() {
        let valid_tokens: Vec<String> = vec![];
        let result = validate_auth_token("any-token", &valid_tokens);
        assert!(result.is_err());
    }

    #[test]
    fn test_case_sensitive() {
        let valid_tokens = vec!["Demo-Token".to_string()];
        assert!(validate_auth_token("demo-token", &valid_tokens).is_err());
        assert!(validate_auth_token("Demo-Token", &valid_tokens).is_ok());
    }
}
