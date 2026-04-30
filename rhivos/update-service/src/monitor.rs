use std::sync::Arc;

use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Monitors a running container by calling `podman wait` and updates
/// the adapter state based on the exit code.
///
/// - Exit code 0 → STOPPED
/// - Exit code non-zero → ERROR
/// - Wait failure → ERROR
pub async fn monitor_container(
    _adapter_id: String,
    _state_mgr: Arc<StateManager>,
    _podman: Arc<dyn PodmanExecutor>,
) {
    todo!("monitor_container not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState};
    use crate::podman::testing::MockPodmanExecutor;
    use tokio::sync::broadcast;

    fn setup_running_adapter(
        adapter_id: &str,
        state_mgr: &StateManager,
    ) {
        let entry = AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: format!("example.com/{adapter_id}:v1"),
            checksum_sha256: "sha256:test".to_string(),
            state: AdapterState::Running,
            job_id: "job-monitor".to_string(),
            stopped_at: None,
            error_message: None,
        };
        state_mgr.create_adapter(entry);
    }

    // TS-07-15: Container Exit Non-Zero Transitions to ERROR
    // Requirement: 07-REQ-9.1
    #[tokio::test]
    async fn test_container_exit_nonzero_error() {
        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock_podman = Arc::new(MockPodmanExecutor::new());
        mock_podman.set_wait_result(Ok(1)); // non-zero exit
        // Direct monitor tests need wait to return immediately.
        mock_podman.set_wait_immediate(true);

        setup_running_adapter("exit-err-v1", &state_mgr);

        monitor_container(
            "exit-err-v1".to_string(),
            state_mgr.clone(),
            mock_podman,
        )
        .await;

        let adapter = state_mgr
            .get_adapter("exit-err-v1")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // TS-07-16: Container Exit Code Zero Transitions to STOPPED
    // Requirement: 07-REQ-9.2
    #[tokio::test]
    async fn test_container_exit_zero_stopped() {
        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock_podman = Arc::new(MockPodmanExecutor::new());
        mock_podman.set_wait_result(Ok(0)); // clean exit
        // Direct monitor tests need wait to return immediately.
        mock_podman.set_wait_immediate(true);

        setup_running_adapter("exit-ok-v1", &state_mgr);

        monitor_container(
            "exit-ok-v1".to_string(),
            state_mgr.clone(),
            mock_podman,
        )
        .await;

        let adapter = state_mgr
            .get_adapter("exit-ok-v1")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Stopped);
    }

    // TS-07-E16: Podman Wait Failure Transitions to ERROR
    // Requirement: 07-REQ-9.E1
    #[tokio::test]
    async fn test_podman_wait_failure_error() {
        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock_podman = Arc::new(MockPodmanExecutor::new());
        mock_podman.set_wait_result(Err(crate::podman::PodmanError::new("connection lost")));
        // Direct monitor tests need wait to return immediately.
        mock_podman.set_wait_immediate(true);

        setup_running_adapter("wait-fail-v1", &state_mgr);

        monitor_container(
            "wait-fail-v1".to_string(),
            state_mgr.clone(),
            mock_podman,
        )
        .await;

        let adapter = state_mgr
            .get_adapter("wait-fail-v1")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
    }
}
