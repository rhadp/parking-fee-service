use crate::podman::PodmanExecutor;
use crate::state::StateManager;
use std::sync::Arc;
use std::time::Duration;

/// Run a single offload check: find STOPPED adapters past the inactivity
/// timeout and clean them up (transition to OFFLOADING, rm, rmi, remove from
/// state).
pub async fn run_offload_check(
    _state_mgr: &Arc<StateManager>,
    _podman: &Arc<dyn PodmanExecutor>,
    _inactivity_timeout: Duration,
) {
    todo!("run_offload_check not yet implemented")
}

/// Spawn a background task that periodically runs offload checks.
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
    use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
    use crate::podman::PodmanError;
    use crate::testing::MockPodmanExecutor;
    use std::time::Instant;
    use tokio::sync::broadcast;

    fn make_stopped_entry(adapter_id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:test".to_string(),
            state: AdapterState::Stopped,
            job_id: "test-job-id".to_string(),
            stopped_at: Some(Instant::now() - Duration::from_secs(3600)),
            error_message: None,
        }
    }

    // TS-07-13: Offload Timer Triggers After Inactivity
    #[tokio::test]
    async fn test_offload_after_timeout() {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock = MockPodmanExecutor::new();
        mock.set_rm_result(Ok(()));
        mock.set_rmi_result(Ok(()));

        let mut rx = state_mgr.subscribe();

        // Create a STOPPED adapter with stopped_at in the past
        let entry = make_stopped_entry("parkhaus-munich-v1.0.0", "example.com/parkhaus-munich:v1.0.0");
        state_mgr.create_adapter(entry);

        // Run offload check with a short timeout (already expired)
        let inactivity_timeout = Duration::from_secs(1);
        run_offload_check(&state_mgr, &(Arc::new(mock.clone()) as Arc<dyn PodmanExecutor>), inactivity_timeout).await;

        // Adapter should be removed
        assert!(
            state_mgr.get_adapter("parkhaus-munich-v1.0.0").is_none(),
            "adapter should be removed after offload"
        );

        // Check podman calls
        assert!(mock.rm_calls().contains(&"parkhaus-munich-v1.0.0".to_string()));
        assert!(mock.rmi_calls().contains(&"example.com/parkhaus-munich:v1.0.0".to_string()));

        // Check for STOPPED->OFFLOADING event
        let event = rx.try_recv().expect("should receive offloading event");
        assert_eq!(event.old_state, AdapterState::Stopped);
        assert_eq!(event.new_state, AdapterState::Offloading);
    }

    // TS-07-E12: Offload Cleanup Failure Transitions to ERROR
    #[tokio::test]
    async fn test_offload_failure_error() {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock = MockPodmanExecutor::new();
        mock.set_rm_result(Ok(()));
        mock.set_rmi_result(Err(PodmanError::new("image in use")));

        let entry = make_stopped_entry("parkhaus-munich-v1.0.0", "example.com/parkhaus-munich:v1.0.0");
        state_mgr.create_adapter(entry);

        let inactivity_timeout = Duration::from_secs(1);
        run_offload_check(&state_mgr, &(Arc::new(mock.clone()) as Arc<dyn PodmanExecutor>), inactivity_timeout).await;

        // Adapter should still exist but in ERROR state
        let adapter = state_mgr
            .get_adapter("parkhaus-munich-v1.0.0")
            .expect("adapter should still exist after failed offload");
        assert_eq!(adapter.state, AdapterState::Error);
    }
}
