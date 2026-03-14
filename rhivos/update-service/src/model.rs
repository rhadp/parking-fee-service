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

impl std::fmt::Display for AdapterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AdapterState::Unknown => "UNKNOWN",
            AdapterState::Downloading => "DOWNLOADING",
            AdapterState::Installing => "INSTALLING",
            AdapterState::Running => "RUNNING",
            AdapterState::Stopped => "STOPPED",
            AdapterState::Error => "ERROR",
            AdapterState::Offloading => "OFFLOADING",
        };
        write!(f, "{}", s)
    }
}

/// In-memory representation of a managed adapter.
#[derive(Clone, Debug)]
pub struct AdapterInfo {
    pub adapter_id: String,
    pub image_ref: String,
    pub checksum: String,
    pub state: AdapterState,
    pub container_id: Option<String>,
    /// Unix timestamp (seconds) when the adapter was created.
    pub created_at: i64,
    /// Unix timestamp (seconds) when the adapter transitioned to STOPPED.
    pub stopped_at: Option<i64>,
    pub error_message: Option<String>,
}

/// A state-transition event emitted to WatchAdapterStates subscribers.
#[derive(Clone, Debug, PartialEq)]
pub struct AdapterStateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    /// Unix timestamp (seconds).
    pub timestamp: i64,
}

/// Derive a human-readable adapter ID from an OCI image reference.
///
/// Extracts the last path segment and replaces the colon separator with a
/// hyphen: `registry/path/name:tag` → `name-tag`.
///
/// # Examples
/// ```
/// use update_service::model::derive_adapter_id;
/// assert_eq!(
///     derive_adapter_id("us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"),
///     "parkhaus-munich-v1.0.0"
/// );
/// ```
pub fn derive_adapter_id(_image_ref: &str) -> String {
    todo!("implement derive_adapter_id")
}

/// Generate a new UUID v4 job ID.
pub fn generate_job_id() -> String {
    todo!("implement generate_job_id")
}

/// Return the current Unix timestamp in seconds.
pub fn now_unix() -> i64 {
    todo!("implement now_unix")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-07-5: Adapter ID Derivation
    #[test]
    fn test_adapter_id_derivation() {
        assert_eq!(
            derive_adapter_id(
                "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"
            ),
            "parkhaus-munich-v1.0.0",
            "last path segment + tag joined with hyphen"
        );
        assert_eq!(
            derive_adapter_id("registry.io/repo/my-adapter:latest"),
            "my-adapter-latest",
            "simple registry/path:tag form"
        );
        assert_eq!(
            derive_adapter_id("simple-image:v2"),
            "simple-image-v2",
            "no registry prefix"
        );
    }

    // TS-07-5: derive_adapter_id is deterministic and non-empty
    #[test]
    fn test_adapter_id_non_empty() {
        let image_ref = "registry.io/org/my-adapter:v1.0";
        let id = derive_adapter_id(image_ref);
        assert!(!id.is_empty(), "adapter_id must not be empty");
    }

    // generate_job_id returns a non-empty string
    #[test]
    fn test_generate_job_id_non_empty() {
        let id = generate_job_id();
        assert!(!id.is_empty(), "job_id must not be empty");
    }

    // generate_job_id returns unique values
    #[test]
    fn test_generate_job_id_unique() {
        let id1 = generate_job_id();
        let id2 = generate_job_id();
        assert_ne!(id1, id2, "consecutive job_ids must be unique");
    }
}

// TS-07-P5: Adapter ID Derivation — property test
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        // TS-07-P5: derive_adapter_id is deterministic and non-empty for any
        // image reference in the expected format.
        #[test]
        #[ignore]
        fn proptest_adapter_id_derivation(
            name in "[a-z][a-z0-9-]{0,15}",
            tag  in "[a-z0-9][a-z0-9._-]{0,10}",
        ) {
            let image_ref = format!("registry.io/path/{}:{}", name, tag);
            let id1 = derive_adapter_id(&image_ref);
            let id2 = derive_adapter_id(&image_ref);
            prop_assert!(!id1.is_empty(), "adapter_id must not be empty");
            prop_assert_eq!(id1, id2, "adapter_id must be deterministic");
        }
    }
}
