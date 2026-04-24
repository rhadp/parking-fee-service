use std::time::Instant;

/// Adapter lifecycle states.
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

/// In-memory representation of an adapter.
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

/// A state transition event emitted to subscribers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterStateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    pub timestamp: u64,
}

/// Derives the adapter ID from an OCI image reference.
///
/// Extracts the last path segment and replaces the colon separating
/// the image name from the tag with a hyphen.
///
/// # Examples
///
/// - `us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0` → `parkhaus-munich-v1.0.0`
/// - `registry.example.com/my-adapter:latest` → `my-adapter-latest`
/// - `simple-image:v2` → `simple-image-v2`
pub fn derive_adapter_id(image_ref: &str) -> String {
    // Extract the last path segment (everything after the final '/')
    let segment = match image_ref.rsplit('/').next() {
        Some(s) => s,
        None => image_ref,
    };
    // Replace the colon separating name from tag with a hyphen
    segment.replace(':', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-07-6: Adapter ID derivation from OCI image reference
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
        assert_eq!(derive_adapter_id("simple-image:v2"), "simple-image-v2");
    }
}
