use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::container::ContainerRuntime;
use crate::oci::{self, OciPuller};
use crate::state::AdapterState;

/// An adapter record tracking its identity, image, state, and activity.
#[derive(Debug, Clone)]
pub struct AdapterRecord {
    pub adapter_id: String,
    pub image_ref: String,
    pub state: AdapterState,
    pub error_message: Option<String>,
    pub last_activity: Instant,
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
    InvalidTransition {
        from: AdapterState,
        to: AdapterState,
    },

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
            adapters: RwLock::new(HashMap::new()),
            event_tx,
            oci_puller,
            container_runtime,
        }
    }

    /// Generate a unix timestamp in seconds.
    fn now_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    /// Derive an adapter_id from an image_ref.
    /// Strips registry prefix and replaces special chars.
    fn derive_adapter_id(image_ref: &str) -> String {
        // Use the last component of the image ref (repo:tag), replacing special chars
        let parts: Vec<&str> = image_ref.rsplitn(2, '/').collect();
        let base = parts[0];
        base.replace([':', '/'], "-")
    }

    /// Transition an adapter's state, validating the transition and emitting an event.
    /// Returns error if the transition is invalid.
    fn transition_state_inner(
        record: &mut AdapterRecord,
        new_state: AdapterState,
        event_tx: &broadcast::Sender<StateEvent>,
    ) -> Result<(), ManagerError> {
        let old_state = record.state;
        if !old_state.can_transition_to(new_state) {
            warn!(
                adapter_id = %record.adapter_id,
                ?old_state,
                ?new_state,
                "rejected invalid state transition"
            );
            return Err(ManagerError::InvalidTransition {
                from: old_state,
                to: new_state,
            });
        }

        record.state = new_state;
        record.last_activity = Instant::now();

        let event = StateEvent {
            adapter_id: record.adapter_id.clone(),
            old_state,
            new_state,
            timestamp: Self::now_timestamp(),
        };

        // Best-effort broadcast; if no receivers, that's fine
        let _ = event_tx.send(event);

        info!(
            adapter_id = %record.adapter_id,
            ?old_state,
            ?new_state,
            "adapter state transition"
        );

        Ok(())
    }

    /// Install an adapter from the given OCI image reference, verifying the checksum.
    ///
    /// This performs the full synchronous lifecycle:
    ///   1. Check if already running (ALREADY_EXISTS)
    ///   2. Stop any currently running adapter (single-adapter constraint)
    ///   3. Create record in UNKNOWN state
    ///   4. Transition to DOWNLOADING, pull image
    ///   5. Verify checksum
    ///   6. Transition to INSTALLING, run container
    ///   7. Transition to RUNNING
    ///
    /// Returns the initial InstallResult with state=DOWNLOADING after step 4 begins,
    /// but the full flow runs synchronously in this implementation for simplicity
    /// with tests. The response reports DOWNLOADING as the initial state.
    pub async fn install_adapter(
        &self,
        image_ref: &str,
        checksum_sha256: &str,
    ) -> Result<InstallResult, ManagerError> {
        let adapter_id = Self::derive_adapter_id(image_ref);
        let job_id = uuid::Uuid::new_v4().to_string();

        // Check if this image_ref is already installed and RUNNING
        {
            let adapters = self.adapters.read().await;
            if let Some(existing) = adapters.values().find(|a| a.image_ref == image_ref) {
                if existing.state == AdapterState::Running {
                    return Err(ManagerError::AlreadyExists(existing.adapter_id.clone()));
                }
            }
        }

        // Single-adapter constraint: stop any currently running adapter
        {
            let mut adapters = self.adapters.write().await;
            let running_ids: Vec<String> = adapters
                .values()
                .filter(|a| a.state == AdapterState::Running)
                .map(|a| a.adapter_id.clone())
                .collect();

            for rid in running_ids {
                if let Some(record) = adapters.get_mut(&rid) {
                    // Stop the container
                    let _ = self.container_runtime.stop(&rid).await;
                    Self::transition_state_inner(record, AdapterState::Stopped, &self.event_tx)?;
                }
            }
        }

        // Create the adapter record in UNKNOWN state
        let mut record = AdapterRecord {
            adapter_id: adapter_id.clone(),
            image_ref: image_ref.to_string(),
            state: AdapterState::Unknown,
            error_message: None,
            last_activity: Instant::now(),
        };

        // Transition to DOWNLOADING
        Self::transition_state_inner(&mut record, AdapterState::Downloading, &self.event_tx)?;

        // Store the record
        {
            let mut adapters = self.adapters.write().await;
            adapters.insert(adapter_id.clone(), record);
        }

        // The install result reports the initial state as DOWNLOADING
        let result = InstallResult {
            job_id,
            adapter_id: adapter_id.clone(),
            state: AdapterState::Downloading,
        };

        // Pull the image
        let pull_result = match self.oci_puller.pull_image(image_ref).await {
            Ok(r) => r,
            Err(e) => {
                // Transition to ERROR
                let mut adapters = self.adapters.write().await;
                if let Some(record) = adapters.get_mut(&adapter_id) {
                    let _ = Self::transition_state_inner(
                        record,
                        AdapterState::Error,
                        &self.event_tx,
                    );
                    record.error_message = Some(e.to_string());
                }
                return Err(ManagerError::RegistryUnavailable(e.to_string()));
            }
        };

        // Verify checksum
        if let Err(e) = oci::verify_checksum(&pull_result.digest, checksum_sha256) {
            // Clean up the image
            let _ = self.oci_puller.remove_image(image_ref).await;
            // Transition to ERROR
            let mut adapters = self.adapters.write().await;
            if let Some(record) = adapters.get_mut(&adapter_id) {
                let _ =
                    Self::transition_state_inner(record, AdapterState::Error, &self.event_tx);
                record.error_message = Some(e.to_string());
            }
            match e {
                oci::OciError::ChecksumMismatch { expected, actual } => {
                    return Err(ManagerError::ChecksumMismatch { expected, actual });
                }
                other => return Err(ManagerError::Internal(other.to_string())),
            }
        }

        // Transition to INSTALLING
        {
            let mut adapters = self.adapters.write().await;
            if let Some(record) = adapters.get_mut(&adapter_id) {
                Self::transition_state_inner(record, AdapterState::Installing, &self.event_tx)?;
            }
        }

        // Run the container
        if let Err(e) = self.container_runtime.run(&adapter_id, image_ref).await {
            let mut adapters = self.adapters.write().await;
            if let Some(record) = adapters.get_mut(&adapter_id) {
                let _ =
                    Self::transition_state_inner(record, AdapterState::Error, &self.event_tx);
                record.error_message = Some(e.to_string());
            }
            return Err(ManagerError::ContainerStartFailed(e.to_string()));
        }

        // Transition to RUNNING
        {
            let mut adapters = self.adapters.write().await;
            if let Some(record) = adapters.get_mut(&adapter_id) {
                Self::transition_state_inner(record, AdapterState::Running, &self.event_tx)?;
            }
        }

        Ok(result)
    }

    /// Remove an adapter by ID, stopping it if running.
    pub async fn remove_adapter(&self, adapter_id: &str) -> Result<(), ManagerError> {
        // Check existence
        {
            let adapters = self.adapters.read().await;
            if !adapters.contains_key(adapter_id) {
                return Err(ManagerError::NotFound(adapter_id.to_string()));
            }
        }

        // Stop if running
        {
            let mut adapters = self.adapters.write().await;
            if let Some(record) = adapters.get_mut(adapter_id) {
                if record.state == AdapterState::Running {
                    let _ = self.container_runtime.stop(adapter_id).await;
                    Self::transition_state_inner(
                        record,
                        AdapterState::Stopped,
                        &self.event_tx,
                    )?;
                }
            }
        }

        // Transition to OFFLOADING
        {
            let mut adapters = self.adapters.write().await;
            if let Some(record) = adapters.get_mut(adapter_id) {
                Self::transition_state_inner(
                    record,
                    AdapterState::Offloading,
                    &self.event_tx,
                )?;
            }
        }

        // Remove the container and the record
        let _ = self.container_runtime.remove(adapter_id).await;
        {
            let mut adapters = self.adapters.write().await;
            adapters.remove(adapter_id);
        }

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
        let adapters = self.adapters.read().await;
        adapters
            .get(adapter_id)
            .cloned()
            .ok_or_else(|| ManagerError::NotFound(adapter_id.to_string()))
    }

    /// Subscribe to state change events.
    pub fn subscribe_state_events(&self) -> broadcast::Receiver<StateEvent> {
        self.event_tx.subscribe()
    }

    /// Offload adapters that have been in STOPPED state longer than
    /// the given `inactivity_timeout`.
    ///
    /// This is called periodically by the `OffloadTimer` background task.
    /// Returns the list of adapter IDs that were offloaded.
    pub async fn offload_inactive_adapters(
        &self,
        inactivity_timeout: std::time::Duration,
    ) -> Vec<String> {
        // Collect adapter IDs eligible for offloading
        let eligible: Vec<String> = {
            let adapters = self.adapters.read().await;
            adapters
                .values()
                .filter(|a| {
                    a.state == AdapterState::Stopped
                        && a.last_activity.elapsed() >= inactivity_timeout
                })
                .map(|a| a.adapter_id.clone())
                .collect()
        };

        let mut offloaded = Vec::new();
        for adapter_id in eligible {
            // Transition to OFFLOADING
            {
                let mut adapters = self.adapters.write().await;
                if let Some(record) = adapters.get_mut(&adapter_id) {
                    // Double-check still STOPPED (may have changed)
                    if record.state != AdapterState::Stopped {
                        continue;
                    }
                    if let Err(e) = Self::transition_state_inner(
                        record,
                        AdapterState::Offloading,
                        &self.event_tx,
                    ) {
                        warn!(adapter_id = %adapter_id, error = %e, "failed to transition to OFFLOADING");
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Remove the container
            let _ = self.container_runtime.remove(&adapter_id).await;

            // Remove from the adapter map
            {
                let mut adapters = self.adapters.write().await;
                adapters.remove(&adapter_id);
            }

            info!(adapter_id = %adapter_id, "adapter offloaded due to inactivity");
            offloaded.push(adapter_id);
        }

        offloaded
    }
}

#[cfg(test)]
#[path = "manager_test.rs"]
mod manager_test;
