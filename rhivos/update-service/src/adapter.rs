use std::time::Instant;

/// Adapter state machine states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdapterState {
    Unknown,
    Downloading,
    Installing,
    Running,
    Stopped,
    Error,
    Offloading,
}

/// In-memory representation of a managed adapter.
#[derive(Clone, Debug)]
pub struct AdapterEntry {
    pub adapter_id: String,
    pub image_ref: String,
    pub checksum_sha256: String,
    pub state: AdapterState,
    pub job_id: String,
    pub stopped_at: Option<Instant>,
    pub error_message: Option<String>,
}

/// Event emitted on every adapter state transition.
#[derive(Clone, Debug)]
pub struct AdapterStateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    pub timestamp: u64,
}

/// Derive a deterministic adapter ID from an OCI image reference.
///
/// Extracts the last path segment and replaces the colon with a hyphen.
/// Example: `us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0`
///          becomes `parkhaus-munich-v1.0.0`.
pub fn derive_adapter_id(image_ref: &str) -> String {
    // Extract the last path segment (after the final '/')
    let last_segment = image_ref
        .rsplit('/')
        .next()
        .unwrap_or(image_ref);
    // Replace colon with hyphen
    last_segment.replace(':', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-07-6: Adapter ID derivation (three cases)
    #[test]
    fn test_derive_adapter_id() {
        assert_eq!(
            derive_adapter_id("us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"),
            "parkhaus-munich-v1.0.0"
        );
        assert_eq!(
            derive_adapter_id("registry.example.com/my-adapter:latest"),
            "my-adapter-latest"
        );
        assert_eq!(derive_adapter_id("simple-image:v2"), "simple-image-v2");
    }

    // TS-07-P1: Adapter ID determinism property test
    #[test]
    #[ignore] // Run with --include-ignored
    fn proptest_adapter_id_determinism() {
        use proptest::prelude::*;

        // Test determinism: same input always produces same output
        proptest!(|(image_ref in "[a-z0-9.-]+/[a-z][a-z0-9-]*:[a-z0-9.]+")| {
            let id1 = derive_adapter_id(&image_ref);
            let id2 = derive_adapter_id(&image_ref);
            prop_assert_eq!(id1, id2);
        });

        // Test uniqueness: different last-segment:tag combos produce different IDs
        proptest!(|(
            prefix in "[a-z0-9.-]+/",
            name1 in "[a-z][a-z0-9-]{0,10}",
            tag1 in "[a-z0-9.]{1,5}",
            name2 in "[a-z][a-z0-9-]{0,10}",
            tag2 in "[a-z0-9.]{1,5}",
        )| {
            let ref1 = format!("{prefix}{name1}:{tag1}");
            let ref2 = format!("{prefix}{name2}:{tag2}");
            let segment1 = format!("{name1}:{tag1}");
            let segment2 = format!("{name2}:{tag2}");
            if segment1 != segment2 {
                prop_assert_ne!(
                    derive_adapter_id(&ref1),
                    derive_adapter_id(&ref2),
                    "Different segments should produce different IDs"
                );
            }
        });
    }
}
