use crate::podman::PodmanExecutor;
use crate::state::StateManager;
use std::sync::Arc;

/// Monitor a running container by calling `podman wait` and updating the
/// adapter state when the container exits.
///
/// - Exit code 0 -> transition to STOPPED.
/// - Exit code non-zero -> transition to ERROR.
/// - Wait failure -> transition to ERROR.
pub async fn monitor_container(
    _adapter_id: &str,
    _state_mgr: &Arc<StateManager>,
    _podman: &Arc<dyn PodmanExecutor>,
) {
    todo!("monitor_container not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
    use crate::podman::PodmanError;
    use crate::testing::MockPodmanExecutor;
    use tokio::sync::broadcast;

    fn make_running_entry(adapter_id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:test".to_string(),
            state: AdapterState::Running,
            job_id: "test-job-id".to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    // TS-07-15: Container Exit Non-Zero Transitions to ERROR
    #[tokio::test]
    async fn test_container_exit_nonzero_error() {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock = MockPodmanExecutor::new();
        mock.set_wait_result(Ok(1)); // non-zero exit

        let entry = make_running_entry("parkhaus-munich-v1.0.0", "example.com/parkhaus-munich:v1.0.0");
        state_mgr.create_adapter(entry);

        monitor_container(
            "parkhaus-munich-v1.0.0",
            &state_mgr,
            &(Arc::new(mock) as Arc<dyn PodmanExecutor>),
        )
        .await;

        let adapter = state_mgr
            .get_adapter("parkhaus-munich-v1.0.0")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // TS-07-16: Container Exit Code Zero Transitions to STOPPED
    #[tokio::test]
    async fn test_container_exit_zero_stopped() {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock = MockPodmanExecutor::new();
        mock.set_wait_result(Ok(0)); // clean exit

        let entry = make_running_entry("parkhaus-munich-v1.0.0", "example.com/parkhaus-munich:v1.0.0");
        state_mgr.create_adapter(entry);

        monitor_container(
            "parkhaus-munich-v1.0.0",
            &state_mgr,
            &(Arc::new(mock) as Arc<dyn PodmanExecutor>),
        )
        .await;

        let adapter = state_mgr
            .get_adapter("parkhaus-munich-v1.0.0")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Stopped);
    }

    // TS-07-E16: Podman Wait Failure Transitions to ERROR
    #[tokio::test]
    async fn test_podman_wait_failure_error() {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock = MockPodmanExecutor::new();
        mock.set_wait_result(Err(PodmanError::new("connection lost")));

        let entry = make_running_entry("parkhaus-munich-v1.0.0", "example.com/parkhaus-munich:v1.0.0");
        state_mgr.create_adapter(entry);

        monitor_container(
            "parkhaus-munich-v1.0.0",
            &state_mgr,
            &(Arc::new(mock) as Arc<dyn PodmanExecutor>),
        )
        .await;

        let adapter = state_mgr
            .get_adapter("parkhaus-munich-v1.0.0")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
    }
}
