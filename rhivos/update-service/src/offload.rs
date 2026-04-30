use std::sync::Arc;
use std::time::Duration;

use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Runs one offload check cycle: finds STOPPED adapters past the timeout
/// and offloads them (rm container + rmi image, then remove from state).
pub async fn run_offload_cycle(
    _state_mgr: Arc<StateManager>,
    _podman: Arc<dyn PodmanExecutor>,
    _inactivity_timeout: Duration,
) {
    todo!("run_offload_cycle not yet implemented")
}

/// Spawns the background offload timer that periodically checks for
/// STOPPED adapters past the inactivity timeout.
pub fn spawn_offload_timer(
    _state_mgr: Arc<StateManager>,
    _podman: Arc<dyn PodmanExecutor>,
    _inactivity_timeout: Duration,
    _check_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    todo!("spawn_offload_timer not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState};
    use crate::podman::testing::MockPodmanExecutor;
    use tokio::sync::broadcast;

    // TS-07-13: Offload Timer Triggers After Inactivity
    // Requirements: 07-REQ-6.1, 07-REQ-6.2, 07-REQ-6.3, 07-REQ-6.4
    #[tokio::test]
    async fn test_offload_after_timeout() {
        let (tx, mut rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock_podman = Arc::new(MockPodmanExecutor::new());
        mock_podman.set_rm_result(Ok(()));
        mock_podman.set_rmi_result(Ok(()));

        // Create a STOPPED adapter with stopped_at in the past.
        let entry = AdapterEntry {
            adapter_id: "offload-v1".to_string(),
            image_ref: "example.com/offload:v1".to_string(),
            checksum_sha256: "sha256:test".to_string(),
            state: AdapterState::Stopped,
            job_id: "job-1".to_string(),
            stopped_at: Some(std::time::Instant::now() - Duration::from_secs(10)),
            error_message: None,
        };
        state_mgr.create_adapter(entry);

        // Run offload with a 1-second timeout — the adapter has been
        // stopped for 10 seconds, so it should be offloaded.
        run_offload_cycle(
            state_mgr.clone(),
            mock_podman.clone(),
            Duration::from_secs(1),
        )
        .await;

        // Adapter should be removed from state.
        assert!(
            state_mgr.get_adapter("offload-v1").is_none(),
            "adapter should be removed after offloading"
        );
        assert!(
            mock_podman.rm_calls().contains(&"offload-v1".to_string()),
            "podman rm should have been called"
        );
        assert!(
            mock_podman
                .rmi_calls()
                .contains(&"example.com/offload:v1".to_string()),
            "podman rmi should have been called"
        );
        // Check that STOPPED->OFFLOADING event was emitted.
        let mut found_offloading = false;
        while let Ok(event) = rx.try_recv() {
            if event.old_state == AdapterState::Stopped
                && event.new_state == AdapterState::Offloading
            {
                found_offloading = true;
            }
        }
        assert!(
            found_offloading,
            "should have emitted STOPPED->OFFLOADING event"
        );
    }

    // TS-07-E12: Offload Cleanup Failure Transitions to ERROR
    // Requirement: 07-REQ-6.E1
    #[tokio::test]
    async fn test_offload_failure_error() {
        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock_podman = Arc::new(MockPodmanExecutor::new());
        mock_podman.set_rm_result(Ok(()));
        mock_podman.set_rmi_result(Err(crate::podman::PodmanError::new("image in use")));

        let entry = AdapterEntry {
            adapter_id: "offload-fail-v1".to_string(),
            image_ref: "example.com/offload-fail:v1".to_string(),
            checksum_sha256: "sha256:test".to_string(),
            state: AdapterState::Stopped,
            job_id: "job-2".to_string(),
            stopped_at: Some(std::time::Instant::now() - Duration::from_secs(10)),
            error_message: None,
        };
        state_mgr.create_adapter(entry);

        run_offload_cycle(
            state_mgr.clone(),
            mock_podman.clone(),
            Duration::from_secs(1),
        )
        .await;

        // Adapter should be in ERROR state (not removed).
        let adapter = state_mgr
            .get_adapter("offload-fail-v1")
            .expect("adapter should still exist");
        assert_eq!(adapter.state, AdapterState::Error);
    }
}
