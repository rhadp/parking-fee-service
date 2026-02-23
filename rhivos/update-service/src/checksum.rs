//! SHA-256 checksum verification for OCI manifests.
//!
//! Stub module — implementation will be added in task group 5.

/// Verify that the SHA-256 digest of `data` matches `expected_hex`.
///
/// Returns `true` if the checksum matches, `false` otherwise.
pub fn verify_checksum(_data: &[u8], _expected_hex: &str) -> bool {
    // Stub: always returns false — real implementation in task group 5
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TS-04-22: SHA-256 checksum verification
    // Requirement: 04-REQ-5.2
    // -----------------------------------------------------------------------

    #[test]
    fn test_checksum_verification_correct() {
        // SHA-256("test manifest content") — pre-computed expected digest.
        // The real implementation will compute SHA-256 of the data and compare.
        let data = b"test manifest content";
        let correct_checksum =
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let result = verify_checksum(data, correct_checksum);
        assert!(result, "correct checksum should verify successfully");
    }

    #[test]
    fn test_checksum_verification_incorrect() {
        let data = b"test manifest content";
        let wrong_checksum =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_checksum(data, wrong_checksum);
        assert!(!result, "wrong checksum should fail verification");
    }
}
