use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::container::ContainerRuntime;
use crate::oci::OciPuller;
use crate::state::AdapterState;

/// Metadata for a managed adapter.
#[derive(Debug, Clone)]
pub struct AdapterRecord {
    pub adapter_id: String,
    pub image_ref: String,
    pub state: AdapterState,
    pub error_message: Option<String>,
    pub last_activity: std::time::Instant,
}

/// Event emitted on adapter state transitions.
#[derive(Debug, Clone)]
pub struct StateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    pub timestamp: i64,
}

/// Manages adapter lifecycle, enforces constraints, and emits state events.
pub struct AdapterManager {
    pub(crate) adapters: Mutex<HashMap<String, AdapterRecord>>,
    pub(crate) event_tx: broadcast::Sender<StateEvent>,
    pub(crate) oci: Arc<dyn OciPuller>,
    pub(crate) container: Arc<dyn ContainerRuntime>,
}

/// Errors from adapter manager operations.
#[derive(Debug, Clone)]
pub enum ManagerError {
    NotFound(String),
    AlreadyExists(String),
    ChecksumMismatch { expected: String, actual: String },
    RegistryUnavailable(String),
    ContainerFailed(String),
    InvalidTransition(String),
    Internal(String),
}

impl std::fmt::Display for ManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagerError::NotFound(msg) => write!(f, "adapter not found: {msg}"),
            ManagerError::AlreadyExists(msg) => {
                write!(f, "adapter already installed and running: {msg}")
            }
            ManagerError::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch: expected {expected}, got {actual}")
            }
            ManagerError::RegistryUnavailable(msg) => {
                write!(f, "failed to pull image: {msg}")
            }
            ManagerError::ContainerFailed(msg) => {
                write!(f, "container failed to start: {msg}")
            }
            ManagerError::InvalidTransition(msg) => {
                write!(f, "invalid state transition: {msg}")
            }
            ManagerError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for ManagerError {}

/// Response from `install_adapter`.
#[derive(Debug, Clone)]
pub struct InstallResult {
    pub job_id: String,
    pub adapter_id: String,
    pub state: AdapterState,
}

impl AdapterManager {
    /// Create a new AdapterManager.
    pub fn new(oci: Arc<dyn OciPuller>, container: Arc<dyn ContainerRuntime>) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            adapters: Mutex::new(HashMap::new()),
            event_tx,
            oci,
            container,
        }
    }

    /// Subscribe to state events.
    pub fn subscribe_state_events(&self) -> broadcast::Receiver<StateEvent> {
        self.event_tx.subscribe()
    }

    /// Install an adapter from an OCI image.
    pub async fn install_adapter(
        &self,
        _image_ref: &str,
        _checksum_sha256: &str,
    ) -> Result<InstallResult, ManagerError> {
        // Stub: not yet implemented
        todo!("install_adapter not yet implemented")
    }

    /// Remove an adapter.
    pub async fn remove_adapter(&self, _adapter_id: &str) -> Result<(), ManagerError> {
        // Stub: not yet implemented
        todo!("remove_adapter not yet implemented")
    }

    /// List all adapters.
    pub async fn list_adapters(&self) -> Vec<AdapterRecord> {
        // Stub: not yet implemented
        todo!("list_adapters not yet implemented")
    }

    /// Get the status of a single adapter.
    pub async fn get_adapter_status(
        &self,
        _adapter_id: &str,
    ) -> Result<AdapterRecord, ManagerError> {
        // Stub: not yet implemented
        todo!("get_adapter_status not yet implemented")
    }
}

#[cfg(test)]
#[path = "manager_test.rs"]
mod tests;
