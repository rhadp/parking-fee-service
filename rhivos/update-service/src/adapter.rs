/// Derives the adapter ID from an OCI image reference.
///
/// Extracts the last path segment and replaces the colon with a hyphen.
/// E.g. `us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0`
/// becomes `parkhaus-munich-v1.0.0`.
pub fn derive_adapter_id(image_ref: &str) -> String {
    // Extract the last path segment (everything after the last '/').
    let last_segment = image_ref
        .rsplit('/')
        .next()
        .unwrap_or(image_ref);
    // Replace the colon separator between name and tag with a hyphen.
    last_segment.replace(':', "-")
}

/// Lifecycle states of an adapter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AdapterState {
    Unknown,
    Downloading,
    Installing,
    Running,
    Stopped,
    Error,
    Offloading,
}

/// In-memory entry tracking a single adapter.
#[derive(Clone, Debug)]
pub struct AdapterEntry {
    pub adapter_id: String,
    pub image_ref: String,
    pub checksum_sha256: String,
    pub state: AdapterState,
    pub job_id: String,
    pub stopped_at: Option<std::time::Instant>,
    pub error_message: Option<String>,
}

/// Event emitted on every adapter state transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterStateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-07-6: Adapter ID Derivation
    // Requirement: 07-REQ-1.6
    // The adapter_id is derived from the image_ref by extracting the last
    // path segment and replacing the colon with a hyphen.
    #[test]
    fn test_derive_adapter_id_full_registry() {
        assert_eq!(
            derive_adapter_id(
                "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"
            ),
            "parkhaus-munich-v1.0.0"
        );
    }

    #[test]
    fn test_derive_adapter_id_short_registry() {
        assert_eq!(
            derive_adapter_id("registry.example.com/my-adapter:latest"),
            "my-adapter-latest"
        );
    }

    #[test]
    fn test_derive_adapter_id_no_registry() {
        assert_eq!(
            derive_adapter_id("simple-image:v2"),
            "simple-image-v2"
        );
    }
}
