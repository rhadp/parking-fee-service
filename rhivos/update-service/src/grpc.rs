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
        _image_ref: &str,
        _checksum_sha256: &str,
    ) -> Result<InstallResponse, ServiceError> {
        todo!()
    }

    /// Remove an adapter (stop if running, remove container and image).
    pub async fn remove_adapter(&self, _adapter_id: &str) -> Result<(), ServiceError> {
        todo!()
    }

    /// Get the current status of an adapter.
    pub async fn get_adapter_status(
        &self,
        _adapter_id: &str,
    ) -> Result<AdapterEntry, ServiceError> {
        todo!()
    }

    /// List all known adapters.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        todo!()
    }

    /// Subscribe to adapter state events.
    pub fn watch_adapter_states(&self) -> broadcast::Receiver<AdapterStateEvent> {
        todo!()
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
