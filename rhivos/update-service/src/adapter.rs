/// Adapter lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AdapterState {
    Unknown,
    Downloading,
    Installing,
    Running,
    Stopped,
    Error,
    Offloading,
}

/// In-memory entry for a managed adapter.
#[derive(Debug, Clone)]
pub struct AdapterEntry {
    pub adapter_id: String,
    pub image_ref: String,
    pub checksum_sha256: String,
    pub state: AdapterState,
    pub job_id: String,
    pub stopped_at: Option<std::time::Instant>,
    pub error_message: Option<String>,
}

/// Event emitted for every state transition.
#[derive(Debug, Clone)]
pub struct AdapterStateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    pub timestamp: u64,
}

/// Derive a deterministic adapter_id from an OCI image reference.
/// Extracts the last path segment and replaces ':' with '-'.
/// E.g., "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0" → "parkhaus-munich-v1.0.0"
pub fn derive_adapter_id(image_ref: &str) -> String {
    // Extract the last path segment (after the last '/')
    let last_segment = image_ref.rsplit('/').next().unwrap_or(image_ref);
    // Replace ':' with '-'
    last_segment.replace(':', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-07-6: Adapter ID derivation from image_ref
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
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(50))]

        // TS-07-P1: Adapter ID determinism
        #[test]
        #[ignore = "proptest: run with --include-ignored"]
        fn proptest_adapter_id_determinism(
            name in "[a-z][a-z0-9-]{0,10}",
            tag in "[a-z0-9][a-z0-9.]{0,10}",
        ) {
            let image_ref = format!("registry.example.com/path/{name}:{tag}");
            let id1 = derive_adapter_id(&image_ref);
            let id2 = derive_adapter_id(&image_ref);
            prop_assert_eq!(id1, id2);
        }

        // TS-07-P1 (part 2): Different last segments produce different IDs
        #[test]
        #[ignore = "proptest: run with --include-ignored"]
        fn proptest_adapter_id_uniqueness(
            name1 in "[a-z][a-z0-9-]{1,10}",
            name2 in "[a-z][a-z0-9-]{1,10}",
            tag in "[a-z0-9][a-z0-9.]{0,10}",
        ) {
            prop_assume!(name1 != name2);
            let ref1 = format!("registry.example.com/{name1}:{tag}");
            let ref2 = format!("registry.example.com/{name2}:{tag}");
            let id1 = derive_adapter_id(&ref1);
            let id2 = derive_adapter_id(&ref2);
            prop_assert_ne!(id1, id2);
        }
    }
}
