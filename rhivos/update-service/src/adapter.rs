use std::time::Instant;

/// Adapter lifecycle states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterState {
    Unknown,
    Downloading,
    Installing,
    Running,
    Stopped,
    Error,
    Offloading,
}

/// In-memory record for a managed adapter.
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterStateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    pub timestamp: u64,
}

/// Derive an adapter ID from an OCI image reference.
///
/// Extracts the last path segment and replaces the colon separator with a
/// hyphen. For example `us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0`
/// becomes `parkhaus-munich-v1.0.0`.
pub fn derive_adapter_id(_image_ref: &str) -> String {
    todo!("derive_adapter_id not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-07-6: Adapter ID Derivation
    #[test]
    fn test_derive_adapter_id_full_registry() {
        assert_eq!(
            derive_adapter_id("us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"),
            "parkhaus-munich-v1.0.0"
        );
    }

    #[test]
    fn test_derive_adapter_id_simple_registry() {
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
