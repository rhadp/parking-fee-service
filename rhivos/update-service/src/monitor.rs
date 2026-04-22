use crate::adapter::AdapterState;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;
use std::sync::Arc;

/// Monitor a running container for exit. Calls `podman wait` and updates
/// adapter state based on the exit code.
///
/// - Exit code 0: transition to STOPPED.
/// - Exit code non-zero: transition to ERROR.
/// - podman wait failure: transition to ERROR.
///
/// Guards against race conditions: if the adapter is no longer in RUNNING
/// state when `podman wait` returns (e.g., another operation stopped or
/// removed it), the monitor does nothing.
pub async fn monitor_container<P: PodmanExecutor>(
    adapter_id: &str,
    state_manager: Arc<StateManager>,
    podman: Arc<P>,
) {
    let exit_result = podman.wait(adapter_id).await;

    // Guard: only transition if the adapter is still in RUNNING state.
    // This prevents races with explicit stop/remove operations that may
    // have already transitioned the adapter while we were waiting.
    let current = state_manager.get_adapter(adapter_id);
    match current {
        Some(entry) if entry.state == AdapterState::Running => {
            match exit_result {
                Ok(0) => {
                    let _ = state_manager.transition(
                        adapter_id,
                        AdapterState::Stopped,
                        None,
                    );
                }
                Ok(code) => {
                    let _ = state_manager.transition(
                        adapter_id,
                        AdapterState::Error,
                        Some(format!("container exited with code {code}")),
                    );
                }
                Err(e) => {
                    let _ = state_manager.transition(
                        adapter_id,
                        AdapterState::Error,
                        Some(e.message),
                    );
                }
            }
        }
        _ => {
            // Adapter no longer RUNNING (was stopped/removed/errored by
            // another operation). Do nothing — the other operation already
            // handled the transition.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::AdapterState;
    use crate::podman::MockPodmanExecutor;

    // -- TS-07-15: Container exit non-zero transitions to ERROR -------------

    #[tokio::test]
    async fn test_container_exit_nonzero_error() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Ok(1)); // non-zero exit code

        let (tx, _rx) = tokio::sync::broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));

        let service = crate::grpc::UpdateServiceImpl::new(
            state_mgr.clone(),
            mock.clone(),
            tx,
        );
        service
            .install_adapter("example.com/adapter:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = state_mgr
            .get_adapter("adapter-v1")
            .expect("adapter should exist");
        assert_eq!(
            adapter.state,
            AdapterState::Error,
            "adapter should be ERROR after non-zero exit"
        );
    }

    // -- TS-07-16: Container exit code 0 transitions to STOPPED -------------

    #[tokio::test]
    async fn test_container_exit_zero_stopped() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Ok(0)); // clean exit

        let (tx, _rx) = tokio::sync::broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));

        let service = crate::grpc::UpdateServiceImpl::new(
            state_mgr.clone(),
            mock.clone(),
            tx,
        );
        service
            .install_adapter("example.com/adapter:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = state_mgr
            .get_adapter("adapter-v1")
            .expect("adapter should exist");
        assert_eq!(
            adapter.state,
            AdapterState::Stopped,
            "adapter should be STOPPED after clean exit"
        );
    }

    // -- TS-07-E16: Podman wait failure transitions to ERROR ----------------

    #[tokio::test]
    async fn test_podman_wait_failure_error() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(crate::podman::PodmanError::new("connection lost")));

        let (tx, _rx) = tokio::sync::broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));

        let service = crate::grpc::UpdateServiceImpl::new(
            state_mgr.clone(),
            mock.clone(),
            tx,
        );
        service
            .install_adapter("example.com/adapter:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = state_mgr
            .get_adapter("adapter-v1")
            .expect("adapter should exist");
        assert_eq!(
            adapter.state,
            AdapterState::Error,
            "adapter should be ERROR after podman wait failure"
        );
    }
}
