//! SHA-256 checksum verification for OCI manifests.
//!
//! Implements 04-REQ-5.2: compute SHA-256 digest and compare against
//! the provided `checksum_sha256`.

use sha2::{Digest, Sha256};

/// Compute the SHA-256 hex digest of the given data.
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex_encode(&result)
}

/// Verify that the SHA-256 digest of `data` matches `expected_hex`.
///
/// Returns `true` if the checksum matches, `false` otherwise.
pub fn verify_checksum(data: &[u8], expected_hex: &str) -> bool {
    let computed = compute_sha256(data);
    computed == expected_hex.to_lowercase()
}

/// Encode bytes as a lowercase hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
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
        // SHA-256("test manifest content") — computed by the implementation.
        let data = b"test manifest content";
        let correct_checksum = compute_sha256(data);
        let result = verify_checksum(data, &correct_checksum);
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

    #[test]
    fn test_compute_sha256_deterministic() {
        let data = b"hello world";
        let hash1 = compute_sha256(data);
        let hash2 = compute_sha256(data);
        assert_eq!(hash1, hash2, "SHA-256 should be deterministic");
        assert_eq!(hash1.len(), 64, "SHA-256 hex digest should be 64 chars");
    }

    #[test]
    fn test_compute_sha256_known_value() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let data = b"";
        let hash = compute_sha256(data);
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
