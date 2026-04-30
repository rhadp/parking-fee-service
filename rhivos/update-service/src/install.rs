use std::sync::Arc;

use crate::adapter::AdapterState;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Validates inputs and initiates the adapter installation flow.
///
/// Returns the `(job_id, adapter_id, initial_state)` triple immediately.
/// The actual pull/verify/run operations happen in a spawned async task.
pub async fn install_adapter(
    _image_ref: &str,
    _checksum_sha256: &str,
    _state_mgr: Arc<StateManager>,
    _podman: Arc<dyn PodmanExecutor>,
) -> Result<(String, String, AdapterState), InstallError> {
    todo!("install_adapter not yet implemented")
}

/// Error returned from the synchronous part of install_adapter.
#[derive(Debug)]
pub enum InstallError {
    InvalidArgument(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
        }
    }
}

impl std::error::Error for InstallError {}

/// Error returned from remove_adapter.
#[derive(Debug)]
pub enum RemoveError {
    /// The adapter was not found in state.
    NotFound(String),
    /// A podman operation failed during removal.
    PodmanFailed(String),
}

impl std::fmt::Display for RemoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoveError::NotFound(msg) => write!(f, "not found: {msg}"),
            RemoveError::PodmanFailed(msg) => write!(f, "podman failed: {msg}"),
        }
    }
}

impl std::error::Error for RemoveError {}

/// Removes an adapter: stops container (if running), removes container
/// and image, and removes from in-memory state.
pub async fn remove_adapter(
    _adapter_id: &str,
    _state_mgr: Arc<StateManager>,
    _podman: Arc<dyn PodmanExecutor>,
) -> Result<(), RemoveError> {
    todo!("remove_adapter not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::podman::testing::MockPodmanExecutor;
    use tokio::sync::broadcast;

    const IMAGE_REF: &str = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    const CHECKSUM: &str = "sha256:abc123";
    const ADAPTER_ID: &str = "parkhaus-munich-v1.0.0";

    fn setup() -> (Arc<StateManager>, Arc<MockPodmanExecutor>) {
        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx));
        let mock_podman = Arc::new(MockPodmanExecutor::new());
        (state_mgr, mock_podman)
    }

    // TS-07-1: InstallAdapter Returns Response Immediately
    // Requirement: 07-REQ-1.1
    #[tokio::test]
    async fn test_install_response_immediate() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Ok(()));
        mock_podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock_podman.set_run_result(Ok(()));

        let (job_id, adapter_id, state) =
            install_adapter(IMAGE_REF, CHECKSUM, state_mgr, mock_podman)
                .await
                .expect("install should succeed");

        // job_id should be a valid UUID v4.
        assert!(
            uuid::Uuid::parse_str(&job_id).is_ok(),
            "job_id should be valid UUID: {job_id}"
        );
        assert_eq!(adapter_id, ADAPTER_ID);
        assert_eq!(state, AdapterState::Downloading);
    }

    // TS-07-2: Podman Pull Executed on Install
    // Requirement: 07-REQ-1.2
    #[tokio::test]
    async fn test_install_calls_podman_pull() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Ok(()));
        mock_podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock_podman.set_run_result(Ok(()));

        install_adapter(IMAGE_REF, CHECKSUM, state_mgr, mock_podman.clone())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(mock_podman.pull_calls(), vec![IMAGE_REF]);
    }

    // TS-07-3: Checksum Verification After Pull
    // Requirement: 07-REQ-1.3
    #[tokio::test]
    async fn test_install_verifies_checksum() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Ok(()));
        mock_podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock_podman.set_run_result(Ok(()));

        install_adapter(IMAGE_REF, CHECKSUM, state_mgr.clone(), mock_podman.clone())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(mock_podman.inspect_digest_calls().len(), 1);
        let adapter = state_mgr.get_adapter(ADAPTER_ID).expect("adapter should exist");
        // After checksum match it should have progressed past Downloading.
        assert!(
            adapter.state == AdapterState::Installing || adapter.state == AdapterState::Running,
            "state should be Installing or Running, got {:?}",
            adapter.state
        );
    }

    // TS-07-4: Container Started With Network Host
    // Requirement: 07-REQ-1.4
    #[tokio::test]
    async fn test_install_runs_with_network_host() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Ok(()));
        mock_podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock_podman.set_run_result(Ok(()));

        install_adapter(IMAGE_REF, CHECKSUM, state_mgr, mock_podman.clone())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(
            mock_podman.run_calls(),
            vec![(ADAPTER_ID.to_string(), IMAGE_REF.to_string())]
        );
    }

    // TS-07-5: State Transitions to RUNNING on Success
    // Requirement: 07-REQ-1.5
    #[tokio::test]
    async fn test_install_reaches_running() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Ok(()));
        mock_podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock_podman.set_run_result(Ok(()));
        // Mock wait() blocks by default (never returns), so the container
        // monitor will not transition the adapter out of RUNNING during
        // the assertion window.

        install_adapter(IMAGE_REF, CHECKSUM, state_mgr.clone(), mock_podman.clone())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = state_mgr
            .get_adapter(ADAPTER_ID)
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Running);
    }

    // TS-07-E1: Empty image_ref Returns INVALID_ARGUMENT
    // Requirement: 07-REQ-1.E1
    #[tokio::test]
    async fn test_install_empty_image_ref() {
        let (state_mgr, mock_podman) = setup();
        let result = install_adapter("", CHECKSUM, state_mgr, mock_podman).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            InstallError::InvalidArgument(msg) => {
                assert!(
                    msg.contains("image_ref is required"),
                    "error message should contain 'image_ref is required', got: {msg}"
                );
            }
        }
    }

    // TS-07-E2: Empty checksum_sha256 Returns INVALID_ARGUMENT
    // Requirement: 07-REQ-1.E2
    #[tokio::test]
    async fn test_install_empty_checksum() {
        let (state_mgr, mock_podman) = setup();
        let result = install_adapter(IMAGE_REF, "", state_mgr, mock_podman).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            InstallError::InvalidArgument(msg) => {
                assert!(
                    msg.contains("checksum_sha256 is required"),
                    "error message should contain 'checksum_sha256 is required', got: {msg}"
                );
            }
        }
    }

    // TS-07-E3: Podman Pull Failure Transitions to ERROR
    // Requirement: 07-REQ-1.E3
    #[tokio::test]
    async fn test_pull_failure_error_state() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Err(crate::podman::PodmanError::new("connection refused")));

        install_adapter(
            "bad-registry.com/img:v1",
            "sha256:abc",
            state_mgr.clone(),
            mock_podman,
        )
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter_id = "img-v1";
        let adapter = state_mgr
            .get_adapter(adapter_id)
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
        assert!(
            adapter
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("connection refused"),
            "error_message should contain stderr, got: {:?}",
            adapter.error_message
        );
    }

    // TS-07-E4: Checksum Mismatch Transitions to ERROR and Removes Image
    // Requirement: 07-REQ-1.E4
    #[tokio::test]
    async fn test_checksum_mismatch_error() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Ok(()));
        mock_podman.set_inspect_result(Ok("sha256:different".to_string()));
        mock_podman.set_rmi_result(Ok(()));

        let image_ref = "example.com/img:v1";
        install_adapter(image_ref, "sha256:expected", state_mgr.clone(), mock_podman.clone())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter_id = "img-v1";
        let adapter = state_mgr
            .get_adapter(adapter_id)
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
        assert!(
            adapter
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("checksum_mismatch"),
            "error_message should contain 'checksum_mismatch', got: {:?}",
            adapter.error_message
        );
        assert!(
            mock_podman.rmi_calls().contains(&image_ref.to_string()),
            "rmi should have been called for the image"
        );
    }

    // TS-07-E5: Podman Run Failure Transitions to ERROR
    // Requirement: 07-REQ-1.E5
    #[tokio::test]
    async fn test_run_failure_error_state() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Ok(()));
        mock_podman.set_inspect_result(Ok("sha256:abc".to_string()));
        mock_podman.set_run_result(Err(crate::podman::PodmanError::new(
            "container create failed",
        )));

        install_adapter(
            "example.com/img:v1",
            "sha256:abc",
            state_mgr.clone(),
            mock_podman,
        )
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter_id = "img-v1";
        let adapter = state_mgr
            .get_adapter(adapter_id)
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // TS-07-7: Single Adapter Constraint Stops Running Adapter
    // Requirements: 07-REQ-2.1, 07-REQ-2.2
    #[tokio::test]
    async fn test_single_adapter_stops_running() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Ok(()));
        mock_podman.set_inspect_result(Ok("sha256:aaa".to_string()));
        mock_podman.set_run_result(Ok(()));
        mock_podman.set_stop_result(Ok(()));

        // Install adapter A.
        let image_ref_a = "example.com/adapter-a:v1";
        install_adapter(image_ref_a, "sha256:aaa", state_mgr.clone(), mock_podman.clone())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter_id_a = "adapter-a-v1";
        let adapter_a = state_mgr.get_adapter(adapter_id_a).expect("A should exist");
        assert_eq!(adapter_a.state, AdapterState::Running);

        // Install adapter B — should stop A first.
        mock_podman.set_inspect_result(Ok("sha256:bbb".to_string()));
        let image_ref_b = "example.com/adapter-b:v1";
        install_adapter(image_ref_b, "sha256:bbb", state_mgr.clone(), mock_podman.clone())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter_id_b = "adapter-b-v1";
        assert!(
            mock_podman.stop_calls().contains(&adapter_id_a.to_string()),
            "should have stopped adapter A"
        );
        let adapter_a = state_mgr.get_adapter(adapter_id_a).expect("A should still exist");
        assert_eq!(adapter_a.state, AdapterState::Stopped);
        let adapter_b = state_mgr.get_adapter(adapter_id_b).expect("B should exist");
        assert_eq!(adapter_b.state, AdapterState::Running);
    }

    // TS-07-E6: Stop Running Adapter Fails But Install Proceeds
    // Requirement: 07-REQ-2.E1
    #[tokio::test]
    async fn test_stop_failure_install_proceeds() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_pull_result(Ok(()));
        mock_podman.set_inspect_result(Ok("sha256:aaa".to_string()));
        mock_podman.set_run_result(Ok(()));

        // Install adapter A.
        let image_ref_a = "example.com/adapter-a:v1";
        install_adapter(image_ref_a, "sha256:aaa", state_mgr.clone(), mock_podman.clone())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Configure stop to fail for adapter A.
        let adapter_id_a = "adapter-a-v1";
        mock_podman.set_stop_result_for(
            adapter_id_a,
            Err(crate::podman::PodmanError::new("timeout")),
        );
        mock_podman.set_inspect_result(Ok("sha256:bbb".to_string()));

        // Install adapter B — stop fails but install proceeds.
        let image_ref_b = "example.com/adapter-b:v1";
        install_adapter(image_ref_b, "sha256:bbb", state_mgr.clone(), mock_podman.clone())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter_id_b = "adapter-b-v1";
        let adapter_a = state_mgr.get_adapter(adapter_id_a).expect("A should exist");
        assert_eq!(adapter_a.state, AdapterState::Error);
        let adapter_b = state_mgr.get_adapter(adapter_id_b).expect("B should exist");
        assert_eq!(adapter_b.state, AdapterState::Running);
    }

    // TS-07-12: RemoveAdapter Cleans Up Container and Image
    // Requirements: 07-REQ-5.1, 07-REQ-5.2
    // Full-stack removal test that verifies podman operations AND state
    // removal. The state.rs test_remove_adapter covers only the state layer.
    #[tokio::test]
    async fn test_remove_adapter_full() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_stop_result(Ok(()));
        mock_podman.set_rm_result(Ok(()));
        mock_podman.set_rmi_result(Ok(()));

        // Create a RUNNING adapter to test removal.
        let entry = crate::adapter::AdapterEntry {
            adapter_id: ADAPTER_ID.to_string(),
            image_ref: IMAGE_REF.to_string(),
            checksum_sha256: CHECKSUM.to_string(),
            state: AdapterState::Running,
            job_id: "job-1".to_string(),
            stopped_at: None,
            error_message: None,
        };
        state_mgr.create_adapter(entry);

        remove_adapter(ADAPTER_ID, state_mgr.clone(), mock_podman.clone())
            .await
            .expect("remove should succeed");

        // Podman should have been called: stop, rm, rmi.
        assert!(
            mock_podman.stop_calls().contains(&ADAPTER_ID.to_string()),
            "podman stop should have been called"
        );
        assert!(
            mock_podman.rm_calls().contains(&ADAPTER_ID.to_string()),
            "podman rm should have been called"
        );
        assert!(
            mock_podman.rmi_calls().contains(&IMAGE_REF.to_string()),
            "podman rmi should have been called"
        );
        // Adapter should be removed from state.
        assert!(
            state_mgr.get_adapter(ADAPTER_ID).is_none(),
            "adapter should be removed from state"
        );
    }

    // TS-07-E11: Podman Removal Failure Returns INTERNAL
    // Requirement: 07-REQ-5.E2
    // Exercises the remove_adapter orchestration function — the gRPC
    // mapping to INTERNAL status is verified in gRPC service tests
    // (task group 5).
    #[tokio::test]
    async fn test_removal_failure_internal() {
        let (state_mgr, mock_podman) = setup();
        mock_podman.set_stop_result(Ok(()));
        mock_podman.set_rm_result(Err(crate::podman::PodmanError::new("container in use")));

        // Create a RUNNING adapter to test removal failure.
        let entry = crate::adapter::AdapterEntry {
            adapter_id: "rm-fail-v1".to_string(),
            image_ref: "example.com/rm-fail:v1".to_string(),
            checksum_sha256: "sha256:test".to_string(),
            state: AdapterState::Running,
            job_id: "job-1".to_string(),
            stopped_at: None,
            error_message: None,
        };
        state_mgr.create_adapter(entry);

        // Remove should fail because podman rm returns an error.
        let result =
            remove_adapter("rm-fail-v1", state_mgr.clone(), mock_podman.clone()).await;
        assert!(result.is_err(), "remove should fail when podman rm fails");

        // Adapter should be in ERROR state (not removed from state).
        let adapter = state_mgr
            .get_adapter("rm-fail-v1")
            .expect("adapter should still exist");
        assert_eq!(adapter.state, AdapterState::Error);
    }
}
