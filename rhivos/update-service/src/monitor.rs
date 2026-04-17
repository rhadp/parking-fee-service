use std::sync::Arc;

use crate::adapter::AdapterState;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Monitor a running container for exit events via `podman wait`.
/// On exit 0: transition to STOPPED. On non-zero: transition to ERROR.
/// On wait failure: transition to ERROR.
pub async fn monitor_container<P: PodmanExecutor + Send + Sync + 'static>(
    adapter_id: String,
    _image_ref: String,
    state: Arc<StateManager>,
    podman: Arc<P>,
) {
    match podman.wait(&adapter_id).await {
        Ok(0) => {
            // Clean container exit: transition to STOPPED and record stopped_at for offload timer.
            // Ignore the result — adapter may have been removed or explicitly stopped already.
            let _ = state.transition(&adapter_id, AdapterState::Stopped, None);
        }
        Ok(exit_code) => {
            // Non-zero exit code: container crashed — transition to ERROR.
            state.force_error(
                &adapter_id,
                &format!("container exited with non-zero code {exit_code}"),
            );
        }
        Err(e) => {
            // podman wait itself failed — transition to ERROR.
            state.force_error(
                &adapter_id,
                &format!("podman wait failed: {}", e.message),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState};
    use crate::podman::{MockPodmanExecutor, PodmanError};
    use crate::state::StateManager;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn make_running_entry(id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: AdapterState::Running,
            job_id: uuid::Uuid::new_v4().to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    // TS-07-15: Container exit non-zero transitions to ERROR
    #[tokio::test]
    async fn test_container_exit_nonzero_error() {
        let (tx, _rx) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx));
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_wait_result(Ok(1)); // non-zero exit

        let entry = make_running_entry(
            "parkhaus-munich-v1.0.0",
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
        );
        state.create_adapter(entry);

        monitor_container(
            "parkhaus-munich-v1.0.0".to_string(),
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0".to_string(),
            state.clone(),
            mock,
        )
        .await;

        let adapter = state.get_adapter("parkhaus-munich-v1.0.0").unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // TS-07-16: Container exit code zero transitions to STOPPED
    #[tokio::test]
    async fn test_container_exit_zero_stopped() {
        let (tx, _rx) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx));
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_wait_result(Ok(0)); // clean exit

        let entry = make_running_entry(
            "parkhaus-munich-v1.0.0",
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
        );
        state.create_adapter(entry);

        monitor_container(
            "parkhaus-munich-v1.0.0".to_string(),
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0".to_string(),
            state.clone(),
            mock,
        )
        .await;

        let adapter = state.get_adapter("parkhaus-munich-v1.0.0").unwrap();
        assert_eq!(adapter.state, AdapterState::Stopped);
    }

    // TS-07-E16: Podman wait failure transitions to ERROR
    #[tokio::test]
    async fn test_podman_wait_failure_error() {
        let (tx, _rx) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx));
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_wait_result(Err(PodmanError::new("connection lost")));

        let entry = make_running_entry(
            "parkhaus-munich-v1.0.0",
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
        );
        state.create_adapter(entry);

        monitor_container(
            "parkhaus-munich-v1.0.0".to_string(),
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0".to_string(),
            state.clone(),
            mock,
        )
        .await;

        let adapter = state.get_adapter("parkhaus-munich-v1.0.0").unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
    }
}
