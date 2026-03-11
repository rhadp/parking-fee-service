use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::container::ContainerRuntime;
use crate::oci::OciPuller;
use crate::state::AdapterState;

/// An adapter record tracking its identity, image, state, and activity.
#[derive(Debug, Clone)]
pub struct AdapterRecord {
    pub adapter_id: String,
    pub image_ref: String,
    pub state: AdapterState,
    pub error_message: Option<String>,
    pub last_activity: std::time::Instant,
}

/// Event emitted when an adapter changes state.
#[derive(Debug, Clone)]
pub struct StateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    pub timestamp: i64,
}

/// Result of a successful install request.
#[derive(Debug, Clone)]
pub struct InstallResult {
    pub job_id: String,
    pub adapter_id: String,
    pub state: AdapterState,
}

/// Error type for manager operations.
#[derive(Debug, thiserror::Error)]
pub enum ManagerError {
    #[error("adapter not found: {0}")]
    NotFound(String),

    #[error("adapter already installed and running: {0}")]
    AlreadyExists(String),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("failed to pull image: {0}")]
    RegistryUnavailable(String),

    #[error("container failed to start: {0}")]
    ContainerStartFailed(String),

    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidTransition { from: AdapterState, to: AdapterState },

    #[error("internal error: {0}")]
    Internal(String),
}

/// The core adapter lifecycle manager.
///
/// Holds all adapter records and coordinates OCI pulling, checksum
/// verification, container management, and state transitions.
pub struct AdapterManager {
    adapters: RwLock<HashMap<String, AdapterRecord>>,
    event_tx: broadcast::Sender<StateEvent>,
    #[allow(dead_code)]
    oci_puller: Arc<dyn OciPuller>,
    #[allow(dead_code)]
    container_runtime: Arc<dyn ContainerRuntime>,
}

impl AdapterManager {
    /// Create a new AdapterManager with the given OCI puller and container runtime.
    pub fn new(
        oci_puller: Arc<dyn OciPuller>,
        container_runtime: Arc<dyn ContainerRuntime>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            adapters: RwLock::new(HashMap::new()),
            event_tx,
            oci_puller,
            container_runtime,
        }
    }

    /// Install an adapter from the given OCI image reference, verifying the checksum.
    pub async fn install_adapter(
        &self,
        _image_ref: &str,
        _checksum_sha256: &str,
    ) -> Result<InstallResult, ManagerError> {
        // Stub: not implemented. Implementation in task group 5.
        Err(ManagerError::Internal("not implemented".into()))
    }

    /// Remove an adapter by ID, stopping it if running.
    pub async fn remove_adapter(&self, _adapter_id: &str) -> Result<(), ManagerError> {
        // Stub: not implemented.
        Err(ManagerError::Internal("not implemented".into()))
    }

    /// List all known adapters.
    pub async fn list_adapters(&self) -> Vec<AdapterRecord> {
        self.adapters.read().await.values().cloned().collect()
    }

    /// Get the status of a specific adapter.
    pub async fn get_adapter_status(
        &self,
        _adapter_id: &str,
    ) -> Result<AdapterRecord, ManagerError> {
        // Stub: not implemented.
        Err(ManagerError::Internal("not implemented".into()))
    }

    /// Subscribe to state change events.
    pub fn subscribe_state_events(&self) -> broadcast::Receiver<StateEvent> {
        self.event_tx.subscribe()
    }
}

#[cfg(test)]
#[path = "manager_test.rs"]
mod manager_test;
