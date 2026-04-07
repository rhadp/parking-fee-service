use crate::adapter::{derive_adapter_id, AdapterEntry, AdapterState};
use crate::config::Config;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;
use std::fmt;
use std::sync::Arc;

/// Response returned by install_adapter.
#[derive(Debug, Clone)]
pub struct InstallResponse {
    pub job_id: String,
    pub adapter_id: String,
    pub state: AdapterState,
}

/// Errors from service operations, mapping to gRPC status codes.
#[derive(Debug)]
pub enum ServiceError {
    InvalidArgument(String),
    NotFound(String),
    Internal(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            ServiceError::NotFound(msg) => write!(f, "not found: {msg}"),
            ServiceError::Internal(msg) => write!(f, "internal: {msg}"),
        }
    }
}

impl std::error::Error for ServiceError {}

/// Core business logic handler for the update service.
pub struct UpdateServiceHandler {
    pub state_mgr: Arc<StateManager>,
    pub podman: Arc<dyn PodmanExecutor>,
    #[allow(dead_code)]
    pub config: Config,
}

impl UpdateServiceHandler {
    pub fn new(
        state_mgr: Arc<StateManager>,
        podman: Arc<dyn PodmanExecutor>,
        config: Config,
    ) -> Self {
        Self {
            state_mgr,
            podman,
            config,
        }
    }

    /// Install an adapter: validate inputs, derive ID, return immediately,
    /// then pull/verify/run in a background task.
    pub async fn install_adapter(
        &self,
        image_ref: &str,
        checksum_sha256: &str,
    ) -> Result<InstallResponse, ServiceError> {
        // Validate inputs
        if image_ref.is_empty() {
            return Err(ServiceError::InvalidArgument(
                "image_ref is required".to_string(),
            ));
        }
        if checksum_sha256.is_empty() {
            return Err(ServiceError::InvalidArgument(
                "checksum_sha256 is required".to_string(),
            ));
        }

        let adapter_id = derive_adapter_id(image_ref);
        let job_id = uuid::Uuid::new_v4().to_string();

        // Single adapter constraint: stop any currently running adapter
        if let Some(running) = self.state_mgr.get_running_adapter() {
            if running.adapter_id != adapter_id {
                match self.podman.stop(&running.adapter_id).await {
                    Ok(()) => {
                        let _ = self.state_mgr.transition(
                            &running.adapter_id,
                            AdapterState::Stopped,
                            None,
                        );
                    }
                    Err(e) => {
                        // 07-REQ-2.E1: stop failure -> ERROR, but proceed
                        let _ = self.state_mgr.transition(
                            &running.adapter_id,
                            AdapterState::Error,
                            Some(e.message.clone()),
                        );
                    }
                }
            }
        }

        // Create adapter entry in UNKNOWN state
        let entry = AdapterEntry {
            adapter_id: adapter_id.clone(),
            image_ref: image_ref.to_string(),
            checksum_sha256: checksum_sha256.to_string(),
            state: AdapterState::Unknown,
            job_id: job_id.clone(),
            stopped_at: None,
            error_message: None,
        };
        self.state_mgr.create_adapter(entry);

        // Transition to DOWNLOADING
        let _ = self
            .state_mgr
            .transition(&adapter_id, AdapterState::Downloading, None);

        let response = InstallResponse {
            job_id,
            adapter_id: adapter_id.clone(),
            state: AdapterState::Downloading,
        };

        // Spawn async background task for pull/verify/run
        let state_mgr = self.state_mgr.clone();
        let podman = self.podman.clone();
        let image_ref_owned = image_ref.to_string();
        let checksum_owned = checksum_sha256.to_string();
        let adapter_id_owned = adapter_id;

        tokio::spawn(async move {
            Self::do_install(
                state_mgr,
                podman,
                &adapter_id_owned,
                &image_ref_owned,
                &checksum_owned,
            )
            .await;
        });

        Ok(response)
    }

    /// Background install: pull, verify checksum, run container.
    async fn do_install(
        state_mgr: Arc<StateManager>,
        podman: Arc<dyn PodmanExecutor>,
        adapter_id: &str,
        image_ref: &str,
        checksum_sha256: &str,
    ) {
        // Step 1: podman pull
        if let Err(e) = podman.pull(image_ref).await {
            let _ = state_mgr.transition(
                adapter_id,
                AdapterState::Error,
                Some(e.message),
            );
            return;
        }

        // Step 2: inspect digest and verify checksum
        match podman.inspect_digest(image_ref).await {
            Ok(digest) => {
                if digest.trim() != checksum_sha256 {
                    // Checksum mismatch: clean up image and error
                    let _ = podman.rmi(image_ref).await;
                    let _ = state_mgr.transition(
                        adapter_id,
                        AdapterState::Error,
                        Some("checksum_mismatch".to_string()),
                    );
                    return;
                }
            }
            Err(e) => {
                let _ = state_mgr.transition(
                    adapter_id,
                    AdapterState::Error,
                    Some(e.message),
                );
                return;
            }
        }

        // Step 3: transition to INSTALLING
        let _ = state_mgr.transition(adapter_id, AdapterState::Installing, None);

        // Step 4: podman run
        if let Err(e) = podman.run(adapter_id, image_ref).await {
            let _ = state_mgr.transition(
                adapter_id,
                AdapterState::Error,
                Some(e.message),
            );
            return;
        }

        // Step 5: transition to RUNNING
        let _ = state_mgr.transition(adapter_id, AdapterState::Running, None);
    }

    /// Remove an adapter: stop if running, rm container, rmi image, remove from state.
    pub async fn remove_adapter(&self, adapter_id: &str) -> Result<(), ServiceError> {
        let entry = self
            .state_mgr
            .get_adapter(adapter_id)
            .ok_or_else(|| ServiceError::NotFound("adapter not found".to_string()))?;

        // Stop if running
        if entry.state == AdapterState::Running {
            if let Err(e) = self.podman.stop(adapter_id).await {
                let _ = self.state_mgr.transition(
                    adapter_id,
                    AdapterState::Error,
                    Some(e.message.clone()),
                );
                return Err(ServiceError::Internal(e.message));
            }
        }

        // Remove container
        if let Err(e) = self.podman.rm(adapter_id).await {
            let _ = self.state_mgr.transition(
                adapter_id,
                AdapterState::Error,
                Some(e.message.clone()),
            );
            return Err(ServiceError::Internal(e.message));
        }

        // Remove image
        if let Err(e) = self.podman.rmi(&entry.image_ref).await {
            let _ = self.state_mgr.transition(
                adapter_id,
                AdapterState::Error,
                Some(e.message.clone()),
            );
            return Err(ServiceError::Internal(e.message));
        }

        // Remove from state
        let _ = self.state_mgr.remove_adapter(adapter_id);
        Ok(())
    }

    /// Get the status of a specific adapter.
    pub fn get_adapter_status(
        &self,
        adapter_id: &str,
    ) -> Result<AdapterEntry, ServiceError> {
        self.state_mgr
            .get_adapter(adapter_id)
            .ok_or_else(|| ServiceError::NotFound("adapter not found".to_string()))
    }

    /// List all known adapters.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        self.state_mgr.list_adapters()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterState, AdapterStateEvent};
    use crate::config::Config;
    use crate::podman::PodmanError;
    use crate::testing::MockPodmanExecutor;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn test_config() -> Config {
        Config {
            grpc_port: 50052,
            registry_url: String::new(),
            inactivity_timeout_secs: 86400,
            container_storage_path: "/tmp/test/".to_string(),
        }
    }

    fn make_service(mock: &MockPodmanExecutor) -> UpdateServiceHandler {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(64);
        let state_mgr = Arc::new(StateManager::new(tx));
        UpdateServiceHandler::new(state_mgr, Arc::new(mock.clone()), test_config())
    }

    const IMAGE_REF: &str = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    const CHECKSUM: &str = "sha256:abc123";

    // TS-07-1: InstallAdapter Returns Response Immediately
    #[tokio::test]
    async fn test_install_response_immediate() {
        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));

        let svc = make_service(&mock);
        let resp = svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();

        // Job ID should be a valid UUID v4
        assert!(
            uuid::Uuid::parse_str(&resp.job_id).is_ok(),
            "job_id should be valid UUID: {}",
            resp.job_id
        );
        assert_eq!(resp.adapter_id, "parkhaus-munich-v1.0.0");
        assert_eq!(resp.state, AdapterState::Downloading);
    }

    // TS-07-2: Podman Pull Executed on Install
    #[tokio::test]
    async fn test_install_calls_podman_pull() {
        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));

        let svc = make_service(&mock);
        let _resp = svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(mock.pull_calls(), vec![IMAGE_REF]);
    }

    // TS-07-3: Checksum Verification After Pull
    #[tokio::test]
    async fn test_install_verifies_checksum() {
        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));

        let svc = make_service(&mock);
        let _resp = svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert!(
            !mock.inspect_digest_calls().is_empty(),
            "should have called inspect_digest"
        );

        let adapter = svc.state_mgr.get_adapter("parkhaus-munich-v1.0.0");
        assert!(adapter.is_some());
        let state = adapter.unwrap().state;
        // After checksum match, state should be Installing or Running
        assert!(
            state == AdapterState::Installing || state == AdapterState::Running,
            "expected Installing or Running, got {state:?}"
        );
    }

    // TS-07-4: Container Started With Network Host
    #[tokio::test]
    async fn test_install_runs_with_network_host() {
        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));

        let svc = make_service(&mock);
        let _resp = svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let run_calls = mock.run_calls();
        assert_eq!(
            run_calls,
            vec![("parkhaus-munich-v1.0.0".to_string(), IMAGE_REF.to_string())]
        );
    }

    // TS-07-5: State Transitions to RUNNING on Success
    #[tokio::test]
    async fn test_install_reaches_running() {
        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));

        let svc = make_service(&mock);
        let _resp = svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = svc
            .state_mgr
            .get_adapter("parkhaus-munich-v1.0.0")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Running);
    }

    // TS-07-E1: Empty image_ref Returns INVALID_ARGUMENT
    #[tokio::test]
    async fn test_install_empty_image_ref() {
        let mock = MockPodmanExecutor::new();
        let svc = make_service(&mock);

        let result = svc.install_adapter("", CHECKSUM).await;
        assert!(result.is_err(), "empty image_ref should fail");
        match result.unwrap_err() {
            ServiceError::InvalidArgument(msg) => {
                assert!(
                    msg.contains("image_ref is required"),
                    "message should say image_ref is required, got: {msg}"
                );
            }
            other => panic!("expected InvalidArgument, got: {other:?}"),
        }
    }

    // TS-07-E2: Empty checksum_sha256 Returns INVALID_ARGUMENT
    #[tokio::test]
    async fn test_install_empty_checksum() {
        let mock = MockPodmanExecutor::new();
        let svc = make_service(&mock);

        let result = svc.install_adapter("example.com/img:v1", "").await;
        assert!(result.is_err(), "empty checksum should fail");
        match result.unwrap_err() {
            ServiceError::InvalidArgument(msg) => {
                assert!(
                    msg.contains("checksum_sha256 is required"),
                    "message should say checksum_sha256 is required, got: {msg}"
                );
            }
            other => panic!("expected InvalidArgument, got: {other:?}"),
        }
    }

    // TS-07-E3: Podman Pull Failure Transitions to ERROR
    #[tokio::test]
    async fn test_pull_failure_error_state() {
        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Err(PodmanError::new("connection refused")));

        let svc = make_service(&mock);
        let _resp = svc
            .install_adapter("bad-registry.com/img:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = svc
            .state_mgr
            .get_adapter("img-v1")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
        assert!(
            adapter
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("connection refused"),
            "error should contain podman stderr"
        );
    }

    // TS-07-E4: Checksum Mismatch Transitions to ERROR and Removes Image
    #[tokio::test]
    async fn test_checksum_mismatch_error() {
        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:different".to_string()));
        mock.set_rmi_result(Ok(()));

        let svc = make_service(&mock);
        let _resp = svc
            .install_adapter("example.com/img:v1", "sha256:expected")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = svc
            .state_mgr
            .get_adapter("img-v1")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
        assert!(
            adapter
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("checksum_mismatch"),
            "error should mention checksum_mismatch"
        );
        assert!(
            mock.rmi_calls().contains(&"example.com/img:v1".to_string()),
            "should remove image on checksum mismatch"
        );
    }

    // TS-07-E5: Podman Run Failure Transitions to ERROR
    #[tokio::test]
    async fn test_run_failure_error_state() {
        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Err(PodmanError::new("container create failed")));

        let svc = make_service(&mock);
        let _resp = svc
            .install_adapter("example.com/img:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = svc
            .state_mgr
            .get_adapter("img-v1")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // TS-07-7: Single Adapter Constraint Stops Running Adapter
    #[tokio::test]
    async fn test_single_adapter_stops_running() {
        let image_ref_a = "example.com/adapter-a:v1";
        let image_ref_b = "example.com/adapter-b:v1";
        let checksum_a = "sha256:aaa";
        let checksum_b = "sha256:bbb";

        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(checksum_a.to_string()));
        mock.set_run_result(Ok(()));

        let svc = make_service(&mock);

        // Install adapter A
        let _resp = svc.install_adapter(image_ref_a, checksum_a).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(
            svc.state_mgr.get_adapter("adapter-a-v1").unwrap().state,
            AdapterState::Running
        );

        // Now install adapter B (should stop A first)
        mock.set_inspect_result(Ok(checksum_b.to_string()));
        let _resp = svc.install_adapter(image_ref_b, checksum_b).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert!(
            mock.stop_calls().contains(&"adapter-a-v1".to_string()),
            "should have stopped adapter A"
        );
        assert_eq!(
            svc.state_mgr.get_adapter("adapter-a-v1").unwrap().state,
            AdapterState::Stopped
        );
        assert_eq!(
            svc.state_mgr.get_adapter("adapter-b-v1").unwrap().state,
            AdapterState::Running
        );
    }

    // TS-07-E6: Stop Running Adapter Fails But Install Proceeds
    #[tokio::test]
    async fn test_stop_failure_install_proceeds() {
        let image_ref_a = "example.com/adapter-a:v1";
        let image_ref_b = "example.com/adapter-b:v1";
        let checksum_a = "sha256:aaa";
        let checksum_b = "sha256:bbb";

        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(checksum_a.to_string()));
        mock.set_run_result(Ok(()));

        let svc = make_service(&mock);

        // Install adapter A
        let _resp = svc.install_adapter(image_ref_a, checksum_a).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Set stop to fail for adapter A
        mock.set_stop_result_for("adapter-a-v1", Err(PodmanError::new("timeout")));
        mock.set_inspect_result(Ok(checksum_b.to_string()));

        // Install adapter B (stop of A fails, but B should still install)
        let _resp = svc.install_adapter(image_ref_b, checksum_b).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert_eq!(
            svc.state_mgr.get_adapter("adapter-a-v1").unwrap().state,
            AdapterState::Error,
            "adapter A should be in ERROR after stop failure"
        );
        assert_eq!(
            svc.state_mgr.get_adapter("adapter-b-v1").unwrap().state,
            AdapterState::Running,
            "adapter B should reach RUNNING"
        );
    }

    // TS-07-E11: Podman Removal Failure Returns INTERNAL
    #[tokio::test]
    async fn test_removal_failure_internal() {
        let mock = MockPodmanExecutor::new();
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));

        let svc = make_service(&mock);
        let _resp = svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Set rm to fail
        mock.set_rm_result(Err(PodmanError::new("container in use")));

        let result = svc.remove_adapter("parkhaus-munich-v1.0.0").await;
        assert!(result.is_err(), "removal should fail");
        match result.unwrap_err() {
            ServiceError::Internal(_) => {}
            other => panic!("expected Internal error, got: {other:?}"),
        }

        let adapter = svc.state_mgr.get_adapter("parkhaus-munich-v1.0.0");
        assert!(adapter.is_some(), "adapter should still exist");
        assert_eq!(adapter.unwrap().state, AdapterState::Error);
    }
}
