use super::*;

/// Helper: compute the expected SHA-256 checksum of a digest string.
fn compute_expected_checksum(digest: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(digest.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

// TS-07-3: Checksum verification with valid checksum
#[test]
fn test_verify_checksum_valid() {
    let digest = "sha256:abc123def456";
    let expected = compute_expected_checksum(digest);

    let result = verify_checksum(digest, &expected);
    assert!(result.is_ok(), "verify_checksum should succeed for matching checksum");
}

// TS-07-E1: Checksum verification with invalid checksum
#[test]
fn test_verify_checksum_mismatch() {
    let digest = "sha256:abc123def456";
    let wrong_checksum = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    let result = verify_checksum(digest, wrong_checksum);
    assert!(result.is_err(), "verify_checksum should fail for mismatched checksum");

    match result.unwrap_err() {
        OciError::ChecksumMismatch { expected, actual } => {
            assert_eq!(expected, wrong_checksum);
            // The actual checksum should be the real SHA-256 of the digest
            let real_checksum = compute_expected_checksum(digest);
            assert_eq!(actual, real_checksum);
        }
        other => panic!("expected ChecksumMismatch, got: {:?}", other),
    }
}

#[test]
fn test_verify_checksum_empty_digest() {
    let digest = "";
    let expected = compute_expected_checksum(digest);

    let result = verify_checksum(digest, &expected);
    assert!(result.is_ok(), "verify_checksum should succeed even with empty digest if checksum matches");
}

#[test]
fn test_verify_checksum_different_digests_produce_different_checksums() {
    let digest_a = "sha256:aaa";
    let digest_b = "sha256:bbb";

    let checksum_a = compute_expected_checksum(digest_a);
    let checksum_b = compute_expected_checksum(digest_b);

    assert_ne!(checksum_a, checksum_b, "different digests should produce different checksums");

    // Cross-verify: a's checksum should not match b's digest
    assert!(verify_checksum(digest_a, &checksum_b).is_err());
    assert!(verify_checksum(digest_b, &checksum_a).is_err());
}

#[test]
fn test_podman_oci_puller_can_be_constructed() {
    let _puller = PodmanOciPuller::new();
    let _puller_default = PodmanOciPuller::default();
}
