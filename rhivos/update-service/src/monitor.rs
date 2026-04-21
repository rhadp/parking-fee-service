use std::sync::Arc;

use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Spawn a task that waits for the named container to exit, then transitions
/// the adapter to STOPPED (exit 0) or ERROR (non-zero / wait failure).
pub async fn monitor_container<P: PodmanExecutor + Send + Sync + 'static>(
    _adapter_id: String,
    _image_ref: String,
    _state_mgr: Arc<StateManager>,
    _podman: Arc<P>,
) {
    todo!("implemented in task group 4")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState};
    use crate::podman::{MockPodmanExecutor, PodmanError};
    use crate::state::StateManager;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn make_running_entry(adapter_id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: AdapterState::Running,
            job_id: "job-1".to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    fn make_state_mgr() -> Arc<StateManager> {
        let (tx, _rx) = broadcast::channel(128);
        Arc::new(StateManager::new(tx))
    }

    // TS-07-15: Container exits with non-zero code → adapter transitions to ERROR
    #[tokio::test]
    async fn test_container_exit_nonzero_error() {
        let sm = make_state_mgr();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        podman.set_wait_result(Ok(1)); // non-zero exit

        sm.create_adapter(make_running_entry(adapter_id, image_ref));
        monitor_container(
            adapter_id.to_string(),
            image_ref.to_string(),
            Arc::clone(&sm),
            Arc::clone(&podman),
        )
        .await;

        let adapter = sm.get_adapter(adapter_id).unwrap();
        assert_eq!(
            adapter.state,
            AdapterState::Error,
            "non-zero exit should transition to ERROR"
        );
    }

    // TS-07-16: Container exits with code 0 → adapter transitions to STOPPED
    #[tokio::test]
    async fn test_container_exit_zero_stopped() {
        let sm = make_state_mgr();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        podman.set_wait_result(Ok(0)); // clean exit

        sm.create_adapter(make_running_entry(adapter_id, image_ref));
        monitor_container(
            adapter_id.to_string(),
            image_ref.to_string(),
            Arc::clone(&sm),
            Arc::clone(&podman),
        )
        .await;

        let adapter = sm.get_adapter(adapter_id).unwrap();
        assert_eq!(
            adapter.state,
            AdapterState::Stopped,
            "exit code 0 should transition to STOPPED"
        );
    }

    // TS-07-E16: Podman wait failure → adapter transitions to ERROR
    #[tokio::test]
    async fn test_podman_wait_failure_error() {
        let sm = make_state_mgr();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        podman.set_wait_result(Err(PodmanError::new("connection lost")));

        sm.create_adapter(make_running_entry(adapter_id, image_ref));
        monitor_container(
            adapter_id.to_string(),
            image_ref.to_string(),
            Arc::clone(&sm),
            Arc::clone(&podman),
        )
        .await;

        let adapter = sm.get_adapter(adapter_id).unwrap();
        assert_eq!(
            adapter.state,
            AdapterState::Error,
            "wait failure should transition to ERROR"
        );
    }
}
