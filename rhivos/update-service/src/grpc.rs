use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
use crate::podman::PodmanExecutor;
use crate::state::StateManager;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Response from the install_adapter operation.
#[derive(Debug)]
pub struct InstallResponse {
    pub job_id: String,
    pub adapter_id: String,
    pub state: AdapterState,
}

/// Service-layer errors mapped to gRPC status codes.
#[derive(Debug)]
pub enum ServiceError {
    InvalidArgument(String),
    NotFound(String),
    Internal(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for ServiceError {}

/// Core service implementation, parameterized by a PodmanExecutor for testability.
pub struct UpdateServiceImpl<P: PodmanExecutor> {
    pub(crate) state_manager: Arc<StateManager>,
    pub(crate) podman: Arc<P>,
    pub(crate) broadcaster: broadcast::Sender<AdapterStateEvent>,
}

impl<P: PodmanExecutor + 'static> UpdateServiceImpl<P> {
    pub fn new(
        state_manager: Arc<StateManager>,
        podman: Arc<P>,
        broadcaster: broadcast::Sender<AdapterStateEvent>,
    ) -> Self {
        Self {
            state_manager,
            podman,
            broadcaster,
        }
    }

    /// Install an adapter from an OCI image reference.
    /// Returns immediately with DOWNLOADING state; actual pull/run happens asynchronously.
    pub async fn install_adapter(
        &self,
        image_ref: &str,
        checksum_sha256: &str,
    ) -> Result<InstallResponse, ServiceError> {
        // Validate inputs (REQ-1.E1, REQ-1.E2).
        if image_ref.is_empty() {
            return Err(ServiceError::InvalidArgument(
                "image_ref is required".to_string(),
            ));
        }
        if checksum_sha256.is_empty() {
            return Err(ServiceError::InvalidArgument(
                "checksum_sha256 is required".to_string(),
            ));
        }

        let adapter_id = crate::adapter::derive_adapter_id(image_ref);
        let job_id = uuid::Uuid::new_v4().to_string();

        // Create adapter entry with initial UNKNOWN state.
        let entry = AdapterEntry {
            adapter_id: adapter_id.clone(),
            image_ref: image_ref.to_string(),
            checksum_sha256: checksum_sha256.to_string(),
            state: AdapterState::Unknown,
            job_id: job_id.clone(),
            stopped_at: None,
            error_message: None,
        };
        self.state_manager.create_adapter(entry);

        // Transition to DOWNLOADING before returning.
        let _ = self
            .state_manager
            .transition(&adapter_id, AdapterState::Downloading, None);

        // Spawn async task for pull → verify → run pipeline.
        let state_mgr = self.state_manager.clone();
        let podman = self.podman.clone();
        let image = image_ref.to_string();
        let checksum = checksum_sha256.to_string();
        let aid = adapter_id.clone();

        tokio::spawn(async move {
            // Single-adapter constraint: stop any currently running adapter (REQ-2.1).
            if let Some(running) = state_mgr.get_running_adapter() {
                if running.adapter_id != aid {
                    match podman.stop(&running.adapter_id).await {
                        Ok(()) => {
                            let _ = state_mgr.transition(
                                &running.adapter_id,
                                AdapterState::Stopped,
                                None,
                            );
                        }
                        Err(e) => {
                            // REQ-2.E1: stop failure → old adapter ERROR, proceed anyway.
                            let _ = state_mgr.transition(
                                &running.adapter_id,
                                AdapterState::Error,
                                Some(e.message.clone()),
                            );
                        }
                    }
                }
            }

            // Pull image (REQ-1.2).
            if let Err(e) = podman.pull(&image).await {
                let _ =
                    state_mgr.transition(&aid, AdapterState::Error, Some(e.message));
                return;
            }

            // Inspect digest and verify checksum (REQ-1.3).
            let digest = match podman.inspect_digest(&image).await {
                Ok(d) => d,
                Err(e) => {
                    let _ = state_mgr.transition(
                        &aid,
                        AdapterState::Error,
                        Some(e.message),
                    );
                    return;
                }
            };

            if digest.trim() != checksum {
                // REQ-1.E4: checksum mismatch → ERROR, remove image.
                let _ = state_mgr.transition(
                    &aid,
                    AdapterState::Error,
                    Some("checksum_mismatch".to_string()),
                );
                let _ = podman.rmi(&image).await;
                return;
            }

            // Transition to INSTALLING (REQ-1.4).
            let _ = state_mgr.transition(&aid, AdapterState::Installing, None);

            // Run container (REQ-1.4).
            if let Err(e) = podman.run(&aid, &image).await {
                let _ =
                    state_mgr.transition(&aid, AdapterState::Error, Some(e.message));
                return;
            }

            // Transition to RUNNING (REQ-1.5).
            let _ = state_mgr.transition(&aid, AdapterState::Running, None);

            // Monitor container exit (REQ-9.1, REQ-9.2, REQ-9.E1).
            // This awaits `podman wait`, so the spawned task stays alive
            // until the container exits. The guard inside monitor_container
            // prevents races with explicit stop/remove operations.
            crate::monitor::monitor_container(&aid, state_mgr, podman).await;
        });

        Ok(InstallResponse {
            job_id,
            adapter_id,
            state: AdapterState::Downloading,
        })
    }

    /// Remove an adapter (stop if running, remove container and image).
    pub async fn remove_adapter(&self, adapter_id: &str) -> Result<(), ServiceError> {
        let entry = self
            .state_manager
            .get_adapter(adapter_id)
            .ok_or_else(|| ServiceError::NotFound("adapter not found".to_string()))?;

        // Stop if currently running (REQ-5.1).
        if entry.state == AdapterState::Running {
            if let Err(e) = self.podman.stop(adapter_id).await {
                let _ = self.state_manager.transition(
                    adapter_id,
                    AdapterState::Error,
                    Some(e.message.clone()),
                );
            }
        }

        // Remove container (REQ-5.1).
        if let Err(e) = self.podman.rm(adapter_id).await {
            let _ = self.state_manager.transition(
                adapter_id,
                AdapterState::Error,
                Some(e.message.clone()),
            );
            return Err(ServiceError::Internal(e.message));
        }

        // Remove image (REQ-5.1).
        if let Err(e) = self.podman.rmi(&entry.image_ref).await {
            let _ = self.state_manager.transition(
                adapter_id,
                AdapterState::Error,
                Some(e.message.clone()),
            );
            return Err(ServiceError::Internal(e.message));
        }

        // Remove from in-memory state (REQ-5.2).
        self.state_manager
            .remove_adapter(adapter_id)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Get the current status of an adapter.
    pub async fn get_adapter_status(
        &self,
        adapter_id: &str,
    ) -> Result<AdapterEntry, ServiceError> {
        self.state_manager
            .get_adapter(adapter_id)
            .ok_or_else(|| ServiceError::NotFound("adapter not found".to_string()))
    }

    /// List all known adapters.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        self.state_manager.list_adapters()
    }

    /// Subscribe to adapter state events.
    pub fn watch_adapter_states(&self) -> broadcast::Receiver<AdapterStateEvent> {
        self.broadcaster.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::podman::MockPodmanExecutor;

    // -- TS-07-E11: Podman removal failure returns INTERNAL -----------------

    #[tokio::test]
    async fn test_removal_failure_internal() {
        let mock = Arc::new(MockPodmanExecutor::new());
        // Set up an adapter in stopped state (via install + stop sequence)
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Ok(()));
        // rm will fail during removal
        mock.set_stop_result(Ok(()));
        mock.set_rm_result(Err(crate::podman::PodmanError::new("container in use")));

        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));
        let service = UpdateServiceImpl::new(state_mgr.clone(), mock, tx);

        // Install adapter first
        service
            .install_adapter("example.com/adapter:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Attempt removal (rm will fail)
        let result = service.remove_adapter("adapter-v1").await;
        assert!(result.is_err(), "remove should fail when podman rm fails");
        let err = result.unwrap_err();
        assert!(
            matches!(err, ServiceError::Internal(_)),
            "error should be Internal, got {err:?}"
        );

        // Adapter should be in ERROR state
        let adapter = state_mgr.get_adapter("adapter-v1");
        assert!(adapter.is_some());
        assert_eq!(adapter.unwrap().state, crate::adapter::AdapterState::Error);
    }
}
