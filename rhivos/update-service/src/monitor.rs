use crate::adapter::AdapterState;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;
use std::sync::Arc;

/// Monitor a running container for exit. Calls `podman wait` and updates
/// adapter state based on the exit code.
///
/// - Exit code 0: transition to STOPPED (REQ-9.2).
/// - Exit code non-zero: transition to ERROR (REQ-9.1).
/// - podman wait failure: transition to ERROR (REQ-9.E1).
///
/// Uses `transition_from` for an atomic check-and-transition: only
/// transitions if the adapter is still in RUNNING state. This eliminates
/// the TOCTOU race that exists with a separate `get_adapter` +
/// `transition` sequence — between the check and the transition,
/// `RemoveAdapter` or the single-adapter stop could have already
/// transitioned (or removed) the adapter.
pub async fn monitor_container<P: PodmanExecutor>(
    adapter_id: &str,
    state_manager: Arc<StateManager>,
    podman: Arc<P>,
) {
    let exit_result = podman.wait(adapter_id).await;

    // Atomically transition from RUNNING to the appropriate state.
    // If the adapter is no longer RUNNING (e.g., RemoveAdapter already
    // transitioned it, or another install stopped it), transition_from
    // returns Err(InvalidTransition) or Err(NotFound), which we
    // silently ignore — the other operation already handled the state.
    match exit_result {
        Ok(0) => {
            let _ = state_manager.transition_from(
                adapter_id,
                AdapterState::Running,
                AdapterState::Stopped,
                None,
            );
        }
        Ok(code) => {
            let _ = state_manager.transition_from(
                adapter_id,
                AdapterState::Running,
                AdapterState::Error,
                Some(format!("container exited with code {code}")),
            );
        }
        Err(e) => {
            let _ = state_manager.transition_from(
                adapter_id,
                AdapterState::Running,
                AdapterState::Error,
                Some(e.message),
            );
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

    // -- Race condition guard: monitor skips transition when adapter
    //    is no longer RUNNING (e.g., RemoveAdapter already handled it).
    //    Validates the atomic transition_from approach (critical review finding).

    #[tokio::test]
    async fn test_monitor_skips_when_adapter_already_stopped() {
        let mock = Arc::new(MockPodmanExecutor::new());

        let (tx, mut rx) = tokio::sync::broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx));

        // Manually create an adapter in STOPPED state (simulating
        // RemoveAdapter having already transitioned it).
        let entry = crate::adapter::AdapterEntry {
            adapter_id: "adapter-v1".to_string(),
            image_ref: "example.com/adapter:v1".to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: AdapterState::Stopped,
            job_id: uuid::Uuid::new_v4().to_string(),
            stopped_at: None,
            error_message: None,
        };
        state_mgr.create_adapter(entry);

        // Drain any creation events.
        while rx.try_recv().is_ok() {}

        // Simulate podman wait returning exit code 0 — the monitor
        // should NOT transition the adapter because it is not RUNNING.
        mock.set_wait_result(Ok(0));
        monitor_container("adapter-v1", state_mgr.clone(), mock).await;

        // Adapter should still be STOPPED, not re-transitioned.
        let adapter = state_mgr
            .get_adapter("adapter-v1")
            .expect("adapter should still exist");
        assert_eq!(
            adapter.state,
            AdapterState::Stopped,
            "monitor should not transition adapter that is already STOPPED"
        );

        // No spurious events should have been emitted.
        assert!(
            rx.try_recv().is_err(),
            "no events should be emitted when monitor skips transition"
        );
    }

    #[tokio::test]
    async fn test_monitor_skips_when_adapter_removed() {
        let mock = Arc::new(MockPodmanExecutor::new());

        let (tx, _rx) = tokio::sync::broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx));

        // No adapter in state — simulates RemoveAdapter having
        // already removed it from in-memory state.
        mock.set_wait_result(Ok(1));
        monitor_container("adapter-v1", state_mgr.clone(), mock).await;

        // Should not panic or create any state.
        assert!(
            state_mgr.get_adapter("adapter-v1").is_none(),
            "monitor should not create state for a removed adapter"
        );
    }
}
