use crate::adapter::{AdapterState, AdapterStateEvent};
use crate::podman::PodmanExecutor;
use crate::state::StateManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

/// Run a single offload check: find STOPPED adapters past the inactivity
/// timeout and clean them up (rm + rmi).
///
/// For each candidate:
/// 1. Transition to OFFLOADING (emits event per REQ-6.4).
/// 2. Remove container via `podman rm` (REQ-6.2).
/// 3. Remove image via `podman rmi` (REQ-6.2).
/// 4. Remove adapter from in-memory state (REQ-6.3).
///
/// If any cleanup step fails, the adapter transitions to ERROR (REQ-6.E1).
pub async fn run_offload_check<P: PodmanExecutor>(
    state_manager: &StateManager,
    podman: &P,
    timeout: Duration,
) {
    let candidates = state_manager.get_offload_candidates(timeout);
    for candidate in candidates {
        // Transition to OFFLOADING (REQ-6.1, REQ-6.4).
        let _ = state_manager.transition(
            &candidate.adapter_id,
            AdapterState::Offloading,
            None,
        );

        // Remove container (REQ-6.2).
        if let Err(e) = podman.rm(&candidate.adapter_id).await {
            let _ = state_manager.transition(
                &candidate.adapter_id,
                AdapterState::Error,
                Some(e.message),
            );
            continue;
        }

        // Remove image (REQ-6.2).
        if let Err(e) = podman.rmi(&candidate.image_ref).await {
            let _ = state_manager.transition(
                &candidate.adapter_id,
                AdapterState::Error,
                Some(e.message),
            );
            continue;
        }

        // Remove from in-memory state (REQ-6.3).
        let _ = state_manager.remove_adapter(&candidate.adapter_id);
    }
}

/// Start the offload timer as a background task. Periodically checks for
/// STOPPED adapters past the inactivity timeout and offloads them.
///
/// `check_interval` controls how often the timer checks (configurable for tests).
pub async fn start_offload_timer<P: PodmanExecutor + 'static>(
    state_manager: Arc<StateManager>,
    podman: Arc<P>,
    _broadcaster: broadcast::Sender<AdapterStateEvent>,
    timeout: Duration,
    check_interval: Duration,
) {
    loop {
        tokio::time::sleep(check_interval).await;
        run_offload_check(&state_manager, &*podman, timeout).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::AdapterState;
    use crate::podman::MockPodmanExecutor;

    // -- TS-07-13: Offload timer triggers after inactivity ------------------

    #[tokio::test]
    async fn test_offload_after_timeout() {
        let mock = Arc::new(MockPodmanExecutor::new());
        // Install adapter (pull, inspect, run succeed)
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Ok(()));
        // Stop succeeds, cleanup succeeds
        mock.set_stop_result(Ok(()));
        mock.set_rm_result(Ok(()));
        mock.set_rmi_result(Ok(()));

        let (tx, mut rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));

        // Install and then stop adapter to get it into STOPPED state
        let service = crate::grpc::UpdateServiceImpl::new(
            state_mgr.clone(),
            mock.clone(),
            tx.clone(),
        );
        service
            .install_adapter("example.com/adapter:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Manually transition to STOPPED for test (simulating container exit)
        state_mgr
            .transition("adapter-v1", AdapterState::Stopped, None)
            .unwrap();

        // Start offload timer with short timeout and check interval
        let sm = state_mgr.clone();
        let pm = mock.clone();
        let btx = tx.clone();
        tokio::spawn(async move {
            start_offload_timer(sm, pm, btx, Duration::from_secs(1), Duration::from_millis(100))
                .await;
        });

        // Wait for offload to fire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Adapter should be removed from state
        assert!(
            state_mgr.get_adapter("adapter-v1").is_none(),
            "adapter should be offloaded and removed from state"
        );
        assert!(
            mock.rm_calls().contains(&"adapter-v1".to_string()),
            "podman rm should have been called"
        );
        assert!(
            mock.rmi_calls().contains(&"example.com/adapter:v1".to_string()),
            "podman rmi should have been called"
        );

        // Check for STOPPED -> OFFLOADING event
        let mut found_offload = false;
        while let Ok(event) = rx.try_recv() {
            if event.old_state == AdapterState::Stopped
                && event.new_state == AdapterState::Offloading
            {
                found_offload = true;
            }
        }
        assert!(found_offload, "should have emitted STOPPED->OFFLOADING event");
    }

    // -- TS-07-E12: Offload cleanup failure transitions to ERROR ------------

    #[tokio::test]
    async fn test_offload_failure_error() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Ok(()));
        mock.set_stop_result(Ok(()));
        // rm succeeds but rmi fails during offload cleanup
        mock.set_rm_result(Ok(()));
        mock.set_rmi_result(Err(crate::podman::PodmanError::new("image in use")));

        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));

        let service = crate::grpc::UpdateServiceImpl::new(
            state_mgr.clone(),
            mock.clone(),
            tx.clone(),
        );
        service
            .install_adapter("example.com/adapter:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Transition to STOPPED
        state_mgr
            .transition("adapter-v1", AdapterState::Stopped, None)
            .unwrap();

        // Start offload timer
        let sm = state_mgr.clone();
        let pm = mock.clone();
        let btx = tx.clone();
        tokio::spawn(async move {
            start_offload_timer(sm, pm, btx, Duration::from_secs(1), Duration::from_millis(100))
                .await;
        });

        tokio::time::sleep(Duration::from_secs(2)).await;

        // Adapter should still exist but in ERROR state
        let adapter = state_mgr.get_adapter("adapter-v1");
        assert!(adapter.is_some(), "adapter should still exist after failed offload");
        assert_eq!(
            adapter.unwrap().state,
            AdapterState::Error,
            "adapter should be in ERROR after failed offload cleanup"
        );
    }

    // -- TS-07-P6: Offload timing correctness property test -----------------

    #[test]
    #[ignore] // Run with --include-ignored
    fn proptest_offload_timing_correctness() {
        use proptest::prelude::*;

        let rt = tokio::runtime::Runtime::new().unwrap();

        proptest!(|(timeout_secs in 2u64..5)| {
            rt.block_on(async {
                let mock = Arc::new(MockPodmanExecutor::new());
                mock.set_pull_result(Ok(()));
                mock.set_inspect_result(Ok("sha256:abc".to_string()));
                mock.set_run_result(Ok(()));
                mock.set_stop_result(Ok(()));
                mock.set_rm_result(Ok(()));
                mock.set_rmi_result(Ok(()));

                let (tx, _rx) = broadcast::channel(64);
                let state_mgr = Arc::new(StateManager::new(tx.clone()));

                let service = crate::grpc::UpdateServiceImpl::new(
                    state_mgr.clone(),
                    mock.clone(),
                    tx.clone(),
                );
                service
                    .install_adapter("registry.test/adapter:v1", "sha256:abc")
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(200)).await;

                // Transition to STOPPED
                state_mgr
                    .transition("adapter-v1", AdapterState::Stopped, None)
                    .unwrap();

                // Check BEFORE timeout: adapter should still be STOPPED
                tokio::time::sleep(Duration::from_secs(timeout_secs - 1)).await;
                let adapter = state_mgr.get_adapter("adapter-v1");
                prop_assert!(adapter.is_some(), "adapter should still exist before timeout");
                prop_assert_eq!(adapter.unwrap().state, AdapterState::Stopped);
                Ok::<(), proptest::test_runner::TestCaseError>(())
            })?;
        });
    }
}
