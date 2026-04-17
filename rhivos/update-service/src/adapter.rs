//! Adapter data types and ID derivation.
//!
//! Defines the runtime model for installed adapter containers and the
//! function that computes a stable adapter_id from an OCI image reference.

#![allow(dead_code)]

/// Lifecycle states of a parking-operator adapter container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterState {
    Unknown,
    Downloading,
    Installing,
    Running,
    Stopped,
    Error,
    Offloading,
}

/// Runtime record for an installed adapter.
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

/// Event emitted when an adapter undergoes a state transition.
#[derive(Debug, Clone)]
pub struct AdapterStateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    /// Unix milliseconds.
    pub timestamp: u64,
}

/// Immediate response from an install request (returned synchronously before
/// the background task completes).
#[allow(dead_code)]
#[derive(Debug)]
pub struct InstallResponse {
    pub job_id: String,
    pub adapter_id: String,
    pub state: AdapterState,
}

/// Derive a stable adapter_id from an OCI image reference.
///
/// Extracts the last path segment (everything after the final '/') and
/// replaces the first ':' with '-'.
///
/// Examples:
///   `us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0`
///   -> `parkhaus-munich-v1.0.0`
pub fn derive_adapter_id(image_ref: &str) -> String {
    // Extract the last path segment (after the final '/')
    let last_segment = image_ref.rsplit('/').next().unwrap_or(image_ref);
    // Replace the first ':' with '-'
    last_segment.replacen(':', "-", 1)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-07-6: derive_adapter_id extracts last path segment, replaces ':' with '-'
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

    /// TS-07-P1: adapter_id derivation is deterministic (property test scaffold)
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_adapter_id_determinism() {
        use proptest::prelude::*;
        proptest!(|(name in "[a-z][a-z0-9-]{0,15}", tag in "[a-z0-9][a-z0-9.]{0,9}")| {
            let image_ref = format!("registry.example.com/path/{name}:{tag}");
            let id1 = derive_adapter_id(&image_ref);
            let id2 = derive_adapter_id(&image_ref);
            prop_assert_eq!(id1, id2);
        });
    }
}
