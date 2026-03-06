use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio::time::Instant;
use tracing::{info, warn};

use crate::container::ContainerRuntime;
use crate::oci::{self, OciPuller};
use crate::state::AdapterState;

/// Metadata for a managed adapter.
#[derive(Debug, Clone)]
pub struct AdapterRecord {
    pub adapter_id: String,
    pub image_ref: String,
    pub state: AdapterState,
    pub error_message: Option<String>,
    pub last_activity: Instant,
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

    /// Derive an adapter_id from an image_ref.
    fn adapter_id_from_image_ref(image_ref: &str) -> String {
        // Use the image name (without registry/tag) as a stable adapter ID
        let name = image_ref
            .rsplit('/')
            .next()
            .unwrap_or(image_ref);
        // Remove the tag portion
        let base = name.split(':').next().unwrap_or(name);
        format!("adapter-{base}")
    }

    /// Transition an adapter's state, validate the transition, and emit an event.
    /// The caller must hold the adapters lock.
    fn transition_state_locked(
        adapters: &mut HashMap<String, AdapterRecord>,
        event_tx: &broadcast::Sender<StateEvent>,
        adapter_id: &str,
        new_state: AdapterState,
    ) -> Result<(), ManagerError> {
        let record = adapters
            .get_mut(adapter_id)
            .ok_or_else(|| ManagerError::NotFound(adapter_id.to_string()))?;

        let old_state = record.state;
        if !old_state.can_transition_to(new_state) {
            warn!(
                "invalid state transition for {}: {:?} -> {:?}",
                adapter_id, old_state, new_state
            );
            return Err(ManagerError::InvalidTransition(format!(
                "{:?} -> {:?}",
                old_state, new_state
            )));
        }

        record.state = new_state;
        record.last_activity = Instant::now();

        let event = StateEvent {
            adapter_id: adapter_id.to_string(),
            old_state,
            new_state,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        };

        // Ignore send errors (no subscribers)
        let _ = event_tx.send(event);

        info!(
            "adapter {} transitioned {:?} -> {:?}",
            adapter_id, old_state, new_state
        );

        Ok(())
    }

    /// Install an adapter from an OCI image.
    ///
    /// This method:
    /// 1. Checks for ALREADY_EXISTS (same image already running)
    /// 2. Enforces single-adapter constraint (stops currently running adapter)
    /// 3. Creates the adapter record in UNKNOWN state
    /// 4. Transitions to DOWNLOADING, pulls image, verifies checksum
    /// 5. Transitions to INSTALLING, starts container
    /// 6. Transitions to RUNNING
    ///
    /// Returns immediately with DOWNLOADING state; the full install
    /// proceeds synchronously in this implementation for simplicity.
    pub async fn install_adapter(
        &self,
        image_ref: &str,
        checksum_sha256: &str,
    ) -> Result<InstallResult, ManagerError> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let adapter_id = Self::adapter_id_from_image_ref(image_ref);

        // Check if already installed and running with same image
        {
            let adapters = self.adapters.lock().await;
            if let Some(record) = adapters.get(&adapter_id) {
                if record.state == AdapterState::Running && record.image_ref == image_ref {
                    return Err(ManagerError::AlreadyExists(adapter_id));
                }
            }
        }

        // Enforce single-adapter constraint: stop any currently running adapter
        {
            let mut adapters = self.adapters.lock().await;
            let running: Vec<String> = adapters
                .iter()
                .filter(|(_, r)| r.state == AdapterState::Running)
                .map(|(id, _)| id.clone())
                .collect();

            for running_id in running {
                info!("stopping running adapter {} for single-adapter constraint", running_id);
                // Transition to STOPPED
                Self::transition_state_locked(
                    &mut adapters,
                    &self.event_tx,
                    &running_id,
                    AdapterState::Stopped,
                )?;
                // Actually stop the container (best effort)
                let _ = self.container.stop(&running_id).await;
            }
        }

        // Create adapter record in UNKNOWN state, then transition to DOWNLOADING
        {
            let mut adapters = self.adapters.lock().await;
            adapters.insert(
                adapter_id.clone(),
                AdapterRecord {
                    adapter_id: adapter_id.clone(),
                    image_ref: image_ref.to_string(),
                    state: AdapterState::Unknown,
                    error_message: None,
                    last_activity: Instant::now(),
                },
            );
            Self::transition_state_locked(
                &mut adapters,
                &self.event_tx,
                &adapter_id,
                AdapterState::Downloading,
            )?;
        }

        // Capture values for the background task
        let image_ref_owned = image_ref.to_string();
        let checksum_owned = checksum_sha256.to_string();
        let adapter_id_clone = adapter_id.clone();
        let oci = Arc::clone(&self.oci);
        let container = Arc::clone(&self.container);
        let adapters_ref = &self.adapters;
        let event_tx = &self.event_tx;

        // Pull image
        let pull_result = oci.pull_image(&image_ref_owned).await;
        match pull_result {
            Err(crate::oci::OciError::RegistryUnavailable(msg)) => {
                let mut adapters = adapters_ref.lock().await;
                let _ = Self::transition_state_locked(
                    &mut adapters,
                    event_tx,
                    &adapter_id_clone,
                    AdapterState::Error,
                );
                if let Some(r) = adapters.get_mut(&adapter_id_clone) {
                    r.error_message = Some(msg.clone());
                }
                return Err(ManagerError::RegistryUnavailable(msg));
            }
            Err(other) => {
                let mut adapters = adapters_ref.lock().await;
                let _ = Self::transition_state_locked(
                    &mut adapters,
                    event_tx,
                    &adapter_id_clone,
                    AdapterState::Error,
                );
                let msg = other.to_string();
                if let Some(r) = adapters.get_mut(&adapter_id_clone) {
                    r.error_message = Some(msg.clone());
                }
                return Err(ManagerError::Internal(msg));
            }
            Ok(pull) => {
                // Verify checksum
                if let Err(oci::OciError::ChecksumMismatch { expected, actual }) =
                    oci::verify_checksum(&pull.digest, &checksum_owned)
                {
                    let mut adapters = adapters_ref.lock().await;
                    let _ = Self::transition_state_locked(
                        &mut adapters,
                        event_tx,
                        &adapter_id_clone,
                        AdapterState::Error,
                    );
                    if let Some(r) = adapters.get_mut(&adapter_id_clone) {
                        r.error_message =
                            Some(format!("checksum mismatch: expected {expected}, got {actual}"));
                    }
                    // Clean up the image
                    let _ = oci.remove_image(&image_ref_owned).await;
                    return Err(ManagerError::ChecksumMismatch { expected, actual });
                }

                // Transition to INSTALLING
                {
                    let mut adapters = adapters_ref.lock().await;
                    Self::transition_state_locked(
                        &mut adapters,
                        event_tx,
                        &adapter_id_clone,
                        AdapterState::Installing,
                    )?;
                }

                // Start container
                if let Err(e) = container.run(&adapter_id_clone, &image_ref_owned).await {
                    let msg = e.to_string();
                    let mut adapters = adapters_ref.lock().await;
                    let _ = Self::transition_state_locked(
                        &mut adapters,
                        event_tx,
                        &adapter_id_clone,
                        AdapterState::Error,
                    );
                    if let Some(r) = adapters.get_mut(&adapter_id_clone) {
                        r.error_message = Some(msg.clone());
                    }
                    return Err(ManagerError::ContainerFailed(msg));
                }

                // Transition to RUNNING
                {
                    let mut adapters = adapters_ref.lock().await;
                    Self::transition_state_locked(
                        &mut adapters,
                        event_tx,
                        &adapter_id_clone,
                        AdapterState::Running,
                    )?;
                }
            }
        }

        Ok(InstallResult {
            job_id,
            adapter_id,
            state: AdapterState::Downloading,
        })
    }

    /// Remove an adapter.
    pub async fn remove_adapter(&self, adapter_id: &str) -> Result<(), ManagerError> {
        {
            let adapters = self.adapters.lock().await;
            if !adapters.contains_key(adapter_id) {
                return Err(ManagerError::NotFound(adapter_id.to_string()));
            }
        }

        // Stop if running
        {
            let mut adapters = self.adapters.lock().await;
            if let Some(record) = adapters.get(adapter_id) {
                if record.state == AdapterState::Running {
                    Self::transition_state_locked(
                        &mut adapters,
                        &self.event_tx,
                        adapter_id,
                        AdapterState::Stopped,
                    )?;
                    let _ = self.container.stop(adapter_id).await;
                }
            }
        }

        // Transition to OFFLOADING
        {
            let mut adapters = self.adapters.lock().await;
            if let Some(record) = adapters.get(adapter_id) {
                if record.state == AdapterState::Stopped {
                    Self::transition_state_locked(
                        &mut adapters,
                        &self.event_tx,
                        adapter_id,
                        AdapterState::Offloading,
                    )?;
                }
            }
        }

        // Remove container
        let _ = self.container.remove(adapter_id).await;

        // Remove from adapter list
        {
            let mut adapters = self.adapters.lock().await;
            adapters.remove(adapter_id);
        }

        Ok(())
    }

    /// List all adapters.
    pub async fn list_adapters(&self) -> Vec<AdapterRecord> {
        let adapters = self.adapters.lock().await;
        adapters.values().cloned().collect()
    }

    /// Offload adapters that have been in STOPPED state longer than the given timeout.
    ///
    /// Returns the list of adapter IDs that were offloaded.
    pub async fn offload_inactive_adapters(
        &self,
        inactivity_timeout: std::time::Duration,
    ) -> Vec<String> {
        // Collect candidates (adapters in STOPPED state past timeout)
        let candidates: Vec<String> = {
            let adapters = self.adapters.lock().await;
            adapters
                .iter()
                .filter(|(_, r)| {
                    r.state == AdapterState::Stopped
                        && r.last_activity.elapsed() >= inactivity_timeout
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        let mut offloaded = Vec::new();
        for adapter_id in candidates {
            info!("offloading inactive adapter {}", adapter_id);

            // Transition to OFFLOADING
            {
                let mut adapters = self.adapters.lock().await;
                if let Some(record) = adapters.get(&adapter_id) {
                    if record.state == AdapterState::Stopped {
                        let _ = Self::transition_state_locked(
                            &mut adapters,
                            &self.event_tx,
                            &adapter_id,
                            AdapterState::Offloading,
                        );
                    }
                }
            }

            // Remove container (best effort)
            let _ = self.container.remove(&adapter_id).await;

            // Remove from adapter list
            {
                let mut adapters = self.adapters.lock().await;
                adapters.remove(&adapter_id);
            }

            offloaded.push(adapter_id);
        }

        offloaded
    }

    /// Get the status of a single adapter.
    pub async fn get_adapter_status(
        &self,
        adapter_id: &str,
    ) -> Result<AdapterRecord, ManagerError> {
        let adapters = self.adapters.lock().await;
        adapters
            .get(adapter_id)
            .cloned()
            .ok_or_else(|| ManagerError::NotFound(adapter_id.to_string()))
    }
}

#[cfg(test)]
#[path = "manager_test.rs"]
mod tests;
