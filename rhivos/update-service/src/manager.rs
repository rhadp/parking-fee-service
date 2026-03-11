use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::container::ContainerRuntime;
use crate::oci::{verify_checksum, OciError, OciPuller};
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
    adapters: Arc<RwLock<HashMap<String, AdapterRecord>>>,
    event_tx: broadcast::Sender<StateEvent>,
    oci_puller: Arc<dyn OciPuller>,
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
            adapters: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            oci_puller,
            container_runtime,
        }
    }

    /// Emit a state transition event and update the adapter record in the map.
    async fn transition(
        adapters: &Arc<RwLock<HashMap<String, AdapterRecord>>>,
        event_tx: &broadcast::Sender<StateEvent>,
        adapter_id: &str,
        new_state: AdapterState,
        error_message: Option<String>,
    ) {
        let mut map = adapters.write().await;
        if let Some(record) = map.get_mut(adapter_id) {
            let old_state = record.state;
            record.state = new_state;
            record.last_activity = std::time::Instant::now();
            if let Some(ref msg) = error_message {
                record.error_message = Some(msg.clone());
            }
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let _ = event_tx.send(StateEvent {
                adapter_id: adapter_id.to_string(),
                old_state,
                new_state,
                timestamp,
            });
        }
    }

    /// Install an adapter from the given OCI image reference, verifying the checksum.
    pub async fn install_adapter(
        &self,
        image_ref: &str,
        checksum_sha256: &str,
    ) -> Result<InstallResult, ManagerError> {
        // Check if the same image is already running (AlreadyExists guard).
        {
            let map = self.adapters.read().await;
            for record in map.values() {
                if record.image_ref == image_ref && record.state == AdapterState::Running {
                    return Err(ManagerError::AlreadyExists(record.adapter_id.clone()));
                }
            }
        }

        // Pull image from OCI registry — can fail immediately.
        let pull_result = self
            .oci_puller
            .pull_image(image_ref)
            .await
            .map_err(|e| ManagerError::RegistryUnavailable(e.to_string()))?;

        // Verify checksum — can fail immediately.
        verify_checksum(&pull_result.digest, checksum_sha256).map_err(|e| match e {
            OciError::ChecksumMismatch { expected, actual } => {
                ManagerError::ChecksumMismatch { expected, actual }
            }
            other => ManagerError::Internal(other.to_string()),
        })?;

        // Generate identifiers.
        let adapter_id = uuid::Uuid::new_v4().to_string();
        let job_id = uuid::Uuid::new_v4().to_string();

        // Stop any currently Running adapters (single-adapter enforcement).
        let running_ids: Vec<String> = {
            let map = self.adapters.read().await;
            map.values()
                .filter(|r| r.state == AdapterState::Running)
                .map(|r| r.adapter_id.clone())
                .collect()
        };

        for running_id in running_ids {
            let _ = self.container_runtime.stop(&running_id).await;
            let mut map = self.adapters.write().await;
            if let Some(record) = map.get_mut(&running_id) {
                let old_state = record.state;
                record.state = AdapterState::Stopped;
                record.last_activity = std::time::Instant::now();
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let _ = self.event_tx.send(StateEvent {
                    adapter_id: running_id.clone(),
                    old_state,
                    new_state: AdapterState::Stopped,
                    timestamp,
                });
            }
        }

        // Insert new adapter record in Downloading state and emit Unknown→Downloading.
        {
            let mut map = self.adapters.write().await;
            map.insert(
                adapter_id.clone(),
                AdapterRecord {
                    adapter_id: adapter_id.clone(),
                    image_ref: image_ref.to_string(),
                    state: AdapterState::Downloading,
                    error_message: None,
                    last_activity: std::time::Instant::now(),
                },
            );
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let _ = self.event_tx.send(StateEvent {
            adapter_id: adapter_id.clone(),
            old_state: AdapterState::Unknown,
            new_state: AdapterState::Downloading,
            timestamp,
        });

        // Spawn background task: Downloading → Installing → Running (or Error).
        let adapters_clone = Arc::clone(&self.adapters);
        let event_tx_clone = self.event_tx.clone();
        let container_runtime_clone = Arc::clone(&self.container_runtime);
        let adapter_id_clone = adapter_id.clone();
        let image_ref_clone = image_ref.to_string();

        tokio::spawn(async move {
            // Transition: Downloading → Installing
            Self::transition(
                &adapters_clone,
                &event_tx_clone,
                &adapter_id_clone,
                AdapterState::Installing,
                None,
            )
            .await;

            // Run the container.
            match container_runtime_clone
                .run(&adapter_id_clone, &image_ref_clone)
                .await
            {
                Ok(()) => {
                    // Transition: Installing → Running
                    Self::transition(
                        &adapters_clone,
                        &event_tx_clone,
                        &adapter_id_clone,
                        AdapterState::Running,
                        None,
                    )
                    .await;
                }
                Err(e) => {
                    // Transition: Installing → Error
                    Self::transition(
                        &adapters_clone,
                        &event_tx_clone,
                        &adapter_id_clone,
                        AdapterState::Error,
                        Some(e.to_string()),
                    )
                    .await;
                }
            }
        });

        Ok(InstallResult {
            job_id,
            adapter_id,
            state: AdapterState::Downloading,
        })
    }

    /// Remove an adapter by ID, stopping it if running.
    pub async fn remove_adapter(&self, adapter_id: &str) -> Result<(), ManagerError> {
        // Verify adapter exists.
        let state = {
            let map = self.adapters.read().await;
            match map.get(adapter_id) {
                Some(r) => r.state,
                None => return Err(ManagerError::NotFound(adapter_id.to_string())),
            }
        };

        // Stop container if it's running.
        if state == AdapterState::Running || state == AdapterState::Installing {
            let _ = self.container_runtime.stop(adapter_id).await;
        }

        // Remove from the adapters map.
        self.adapters.write().await.remove(adapter_id);

        Ok(())
    }

    /// List all known adapters.
    pub async fn list_adapters(&self) -> Vec<AdapterRecord> {
        self.adapters.read().await.values().cloned().collect()
    }

    /// Get the status of a specific adapter.
    pub async fn get_adapter_status(
        &self,
        adapter_id: &str,
    ) -> Result<AdapterRecord, ManagerError> {
        let map = self.adapters.read().await;
        map.get(adapter_id)
            .cloned()
            .ok_or_else(|| ManagerError::NotFound(adapter_id.to_string()))
    }

    /// Subscribe to state change events.
    pub fn subscribe_state_events(&self) -> broadcast::Receiver<StateEvent> {
        self.event_tx.subscribe()
    }
}

#[cfg(test)]
#[path = "manager_test.rs"]
mod manager_test;
