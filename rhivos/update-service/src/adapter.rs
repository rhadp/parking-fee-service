use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum AdapterState {
    Unknown,
    Downloading,
    Installing,
    Running,
    Stopped,
    Error,
    Offloading,
}

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

#[derive(Clone, Debug)]
pub struct AdapterStateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    pub timestamp: u64,
}

/// Derive adapter_id from OCI image reference.
/// Extracts the last path segment and replaces ':' with '-'.
/// E.g. "registry/path/name:tag" -> "name-tag"
pub fn derive_adapter_id(_image_ref: &str) -> String {
    todo!("implemented in task group 2")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-07-6: Three canonical cases for adapter_id derivation
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
    #[ignore]
    fn proptest_adapter_id_determinism() {
        use proptest::prelude::*;
        proptest!(|(name in "[a-z][a-z0-9-]{0,20}", tag in "[a-z0-9][a-z0-9.]{0,10}")| {
            let image_ref = format!("registry.example.com/path/{name}:{tag}");
            let id1 = derive_adapter_id(&image_ref);
            let id2 = derive_adapter_id(&image_ref);
            prop_assert_eq!(id1, id2);
        });
    }

    // TS-07-P1: Adapter ID uniqueness property test
    #[test]
    #[ignore]
    fn proptest_adapter_id_uniqueness() {
        use proptest::prelude::*;
        proptest!(|(
            name1 in "[a-z][a-z0-9-]{0,20}",
            tag1 in "[a-z0-9][a-z0-9.]{0,10}",
            name2 in "[a-z][a-z0-9-]{0,20}",
            tag2 in "[a-z0-9][a-z0-9.]{0,10}"
        )| {
            prop_assume!(format!("{name1}:{tag1}") != format!("{name2}:{tag2}"));
            let ref1 = format!("registry.example.com/{name1}:{tag1}");
            let ref2 = format!("registry.example.com/{name2}:{tag2}");
            let id1 = derive_adapter_id(&ref1);
            let id2 = derive_adapter_id(&ref2);
            prop_assert_ne!(id1, id2);
        });
    }
}
