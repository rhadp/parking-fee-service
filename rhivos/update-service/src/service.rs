use std::sync::Arc;
use tokio::sync::broadcast;

use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Response returned by install_adapter.
#[derive(Debug)]
pub struct InstallResponse {
    pub job_id: String,
    pub adapter_id: String,
    pub state: AdapterState,
}

/// Error variants returned by service operations.
#[derive(Debug)]
pub enum ServiceError {
    InvalidArgument(String),
    NotFound(String),
    Internal(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::InvalidArgument(msg) => write!(f, "InvalidArgument: {msg}"),
            ServiceError::NotFound(msg) => write!(f, "NotFound: {msg}"),
            ServiceError::Internal(msg) => write!(f, "Internal: {msg}"),
        }
    }
}

impl std::error::Error for ServiceError {}

/// Core service orchestrating adapter lifecycle.
pub struct UpdateService<P: PodmanExecutor + Send + Sync + 'static> {
    #[allow(dead_code)]
    pub(crate) state_mgr: Arc<StateManager>,
    #[allow(dead_code)]
    pub(crate) podman: Arc<P>,
    #[allow(dead_code)]
    pub(crate) broadcaster: broadcast::Sender<AdapterStateEvent>,
}

impl<P: PodmanExecutor + Send + Sync + 'static> UpdateService<P> {
    pub fn new(
        _state_mgr: Arc<StateManager>,
        _podman: Arc<P>,
        _broadcaster: broadcast::Sender<AdapterStateEvent>,
    ) -> Self {
        todo!("implemented in task group 3")
    }

    /// Validate inputs, derive adapter_id, stop any running adapter, create
    /// entry, return immediately while spawning async install task.
    pub async fn install_adapter(
        &self,
        _image_ref: &str,
        _checksum_sha256: &str,
    ) -> Result<InstallResponse, ServiceError> {
        todo!("implemented in task group 3")
    }

    /// Stop (if running), rm, rmi the adapter, then remove from state.
    pub async fn remove_adapter(&self, _adapter_id: &str) -> Result<(), ServiceError> {
        todo!("implemented in task group 5")
    }

    /// Return all known adapters.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        todo!("implemented in task group 5")
    }

    /// Return state for a specific adapter, or NotFound.
    pub fn get_adapter_status(&self, _adapter_id: &str) -> Result<AdapterEntry, ServiceError> {
        todo!("implemented in task group 5")
    }

    /// Subscribe to adapter state change events.
    pub fn subscribe(&self) -> broadcast::Receiver<AdapterStateEvent> {
        todo!("implemented in task group 5")
    }
}

// ---------------------------------------------------------------------------
// Service-level tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::podman::MockPodmanExecutor;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    const IMAGE_REF_A: &str = "registry.example.com/adapter-a:v1";
    const IMAGE_REF_B: &str = "registry.example.com/adapter-b:v2";
    const CHECKSUM_A: &str = "sha256:aaa111";
    const CHECKSUM_B: &str = "sha256:bbb222";
    const ADAPTER_ID_A: &str = "adapter-a-v1";
    const ADAPTER_ID_B: &str = "adapter-b-v2";

    fn make_service() -> (
        Arc<StateManager>,
        Arc<MockPodmanExecutor>,
        UpdateService<MockPodmanExecutor>,
    ) {
        let (tx, _rx) = broadcast::channel(128);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));
        let podman = Arc::new(MockPodmanExecutor::new());
        let svc = UpdateService::new(Arc::clone(&state_mgr), Arc::clone(&podman), tx);
        (state_mgr, podman, svc)
    }

    // TS-07-7: Installing a second adapter stops the running one first
    #[tokio::test]
    async fn test_single_adapter_stops_running() {
        let (sm, podman, svc) = make_service();
        // Set up all successes
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok(CHECKSUM_A.to_string()));
        podman.set_run_result(Ok(()));

        // Install adapter A
        svc.install_adapter(IMAGE_REF_A, CHECKSUM_A).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(
            sm.get_adapter(ADAPTER_ID_A).unwrap().state,
            AdapterState::Running
        );

        // Now reconfigure for adapter B
        podman.set_inspect_result(Ok(CHECKSUM_B.to_string()));

        // Install adapter B — should stop adapter A first
        svc.install_adapter(IMAGE_REF_B, CHECKSUM_B).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert!(
            podman.stop_calls().contains(&ADAPTER_ID_A.to_string()),
            "expected stop call for adapter A"
        );
        assert_eq!(
            sm.get_adapter(ADAPTER_ID_A).unwrap().state,
            AdapterState::Stopped
        );
        assert_eq!(
            sm.get_adapter(ADAPTER_ID_B).unwrap().state,
            AdapterState::Running
        );
    }

    // TS-07-E6: Stop failure for old adapter transitions it to ERROR but new install proceeds
    #[tokio::test]
    async fn test_stop_failure_install_proceeds() {
        use crate::podman::PodmanError;

        let (sm, podman, svc) = make_service();
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok(CHECKSUM_A.to_string()));
        podman.set_run_result(Ok(()));

        // Install adapter A to RUNNING
        svc.install_adapter(IMAGE_REF_A, CHECKSUM_A).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(
            sm.get_adapter(ADAPTER_ID_A).unwrap().state,
            AdapterState::Running
        );

        // Make stop fail for adapter A
        podman.set_stop_result_for(ADAPTER_ID_A, Err(PodmanError::new("timeout")));
        podman.set_inspect_result(Ok(CHECKSUM_B.to_string()));

        // Install adapter B
        svc.install_adapter(IMAGE_REF_B, CHECKSUM_B).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert_eq!(
            sm.get_adapter(ADAPTER_ID_A).unwrap().state,
            AdapterState::Error
        );
        assert_eq!(
            sm.get_adapter(ADAPTER_ID_B).unwrap().state,
            AdapterState::Running
        );
    }

    // TS-07-E11: Podman removal failure returns Internal and transitions to ERROR
    #[tokio::test]
    async fn test_removal_failure_internal() {
        use crate::podman::PodmanError;

        let (sm, podman, svc) = make_service();
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok(CHECKSUM_A.to_string()));
        podman.set_run_result(Ok(()));

        svc.install_adapter(IMAGE_REF_A, CHECKSUM_A).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        podman.set_rm_result(Err(PodmanError::new("container in use")));
        let result = svc.remove_adapter(ADAPTER_ID_A).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Internal(_)));
        assert_eq!(
            sm.get_adapter(ADAPTER_ID_A).unwrap().state,
            AdapterState::Error
        );
    }

    // TS-07-E8 (service layer): get_adapter_status for unknown ID returns NotFound
    #[test]
    fn test_get_unknown_adapter_service() {
        let (_sm, _podman, svc) = make_service();
        let result = svc.get_adapter_status("nonexistent-adapter");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::NotFound(_)));
    }

    // TS-07-E10 (service layer): remove_adapter for unknown ID returns NotFound
    #[tokio::test]
    async fn test_remove_unknown_adapter_service() {
        let (_sm, _podman, svc) = make_service();
        let result = svc.remove_adapter("nonexistent-adapter").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::NotFound(_)));
    }

    // TS-07-P2: Single adapter invariant property test
    #[test]
    #[ignore]
    fn proptest_single_adapter_invariant() {
        // At most one adapter RUNNING at any time across any sequence of installs
        // Implemented as part of task group 3 verification
    }

    // TS-07-P4: Event delivery completeness property test (service level)
    #[test]
    #[ignore]
    fn proptest_event_delivery_completeness_service() {
        // All active subscribers receive the same events
        // Implemented as part of task group 3 verification
    }
}
