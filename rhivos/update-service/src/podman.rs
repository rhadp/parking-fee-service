use async_trait::async_trait;

/// Error type for podman CLI operations.
#[derive(Debug, Clone)]
pub struct PodmanError {
    pub message: String,
}

impl PodmanError {
    pub fn new(msg: &str) -> Self {
        Self {
            message: msg.to_string(),
        }
    }
}

impl std::fmt::Display for PodmanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "podman error: {}", self.message)
    }
}

impl std::error::Error for PodmanError {}

/// Real podman executor that shells out to the podman CLI via
/// `tokio::process::Command`.
pub struct RealPodmanExecutor;

#[async_trait]
impl PodmanExecutor for RealPodmanExecutor {
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["pull", image_ref])
            .output()
            .await
            .map_err(|e| PodmanError::new(&e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::new(stderr.trim()));
        }
        Ok(())
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["image", "inspect", "--format", "{{.Digest}}", image_ref])
            .output()
            .await
            .map_err(|e| PodmanError::new(&e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::new(stderr.trim()));
        }
        let digest = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(digest)
    }

    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args([
                "run",
                "-d",
                "--name",
                adapter_id,
                "--network=host",
                image_ref,
            ])
            .output()
            .await
            .map_err(|e| PodmanError::new(&e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::new(stderr.trim()));
        }
        Ok(())
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["stop", adapter_id])
            .output()
            .await
            .map_err(|e| PodmanError::new(&e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::new(stderr.trim()));
        }
        Ok(())
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["rm", adapter_id])
            .output()
            .await
            .map_err(|e| PodmanError::new(&e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::new(stderr.trim()));
        }
        Ok(())
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["rmi", image_ref])
            .output()
            .await
            .map_err(|e| PodmanError::new(&e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::new(stderr.trim()));
        }
        Ok(())
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["wait", adapter_id])
            .output()
            .await
            .map_err(|e| PodmanError::new(&e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::new(stderr.trim()));
        }
        let code_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let code = code_str
            .parse::<i32>()
            .map_err(|e| PodmanError::new(&format!("failed to parse exit code: {e}")))?;
        Ok(code)
    }
}

/// Trait abstracting podman CLI operations for testability.
#[async_trait]
pub trait PodmanExecutor: Send + Sync {
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError>;
    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError>;
    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError>;
    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError>;
    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError>;
    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError>;
    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError>;
}

// ---------------------------------------------------------------------------
// Mock executor for unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub struct MockPodmanExecutor {
    inner: std::sync::Mutex<MockState>,
}

#[cfg(test)]
#[derive(Default)]
struct MockState {
    pull_results: std::collections::VecDeque<Result<(), PodmanError>>,
    inspect_results: std::collections::VecDeque<Result<String, PodmanError>>,
    run_results: std::collections::VecDeque<Result<(), PodmanError>>,
    stop_results: std::collections::VecDeque<Result<(), PodmanError>>,
    rm_results: std::collections::VecDeque<Result<(), PodmanError>>,
    rmi_results: std::collections::VecDeque<Result<(), PodmanError>>,
    wait_results: std::collections::VecDeque<Result<i32, PodmanError>>,

    pull_calls: Vec<String>,
    inspect_calls: Vec<String>,
    run_calls: Vec<(String, String)>,
    stop_calls: Vec<String>,
    rm_calls: Vec<String>,
    rmi_calls: Vec<String>,
    wait_calls: Vec<String>,
}

#[cfg(test)]
impl MockPodmanExecutor {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(MockState::default()),
        }
    }

    // --- Result setters (enqueue results for sequential calls) ---

    pub fn set_pull_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().pull_results.push_back(result);
    }

    pub fn set_inspect_result(&self, result: Result<String, PodmanError>) {
        self.inner.lock().unwrap().inspect_results.push_back(result);
    }

    pub fn set_run_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().run_results.push_back(result);
    }

    pub fn set_stop_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().stop_results.push_back(result);
    }

    pub fn set_rm_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().rm_results.push_back(result);
    }

    pub fn set_rmi_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().rmi_results.push_back(result);
    }

    pub fn set_wait_result(&self, result: Result<i32, PodmanError>) {
        self.inner.lock().unwrap().wait_results.push_back(result);
    }

    // --- Call tracking getters ---

    pub fn pull_calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().pull_calls.clone()
    }

    pub fn inspect_calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().inspect_calls.clone()
    }

    pub fn run_calls(&self) -> Vec<(String, String)> {
        self.inner.lock().unwrap().run_calls.clone()
    }

    pub fn stop_calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().stop_calls.clone()
    }

    pub fn rm_calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().rm_calls.clone()
    }

    pub fn rmi_calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().rmi_calls.clone()
    }

    pub fn wait_calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().wait_calls.clone()
    }
}

#[cfg(test)]
#[async_trait]
impl PodmanExecutor for MockPodmanExecutor {
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
        let mut state = self.inner.lock().unwrap();
        state.pull_calls.push(image_ref.to_string());
        state.pull_results.pop_front().unwrap_or(Ok(()))
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        let mut state = self.inner.lock().unwrap();
        state.inspect_calls.push(image_ref.to_string());
        state
            .inspect_results
            .pop_front()
            .unwrap_or(Ok(String::new()))
    }

    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
        let mut state = self.inner.lock().unwrap();
        state
            .run_calls
            .push((adapter_id.to_string(), image_ref.to_string()));
        state.run_results.pop_front().unwrap_or(Ok(()))
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let mut state = self.inner.lock().unwrap();
        state.stop_calls.push(adapter_id.to_string());
        state.stop_results.pop_front().unwrap_or(Ok(()))
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let mut state = self.inner.lock().unwrap();
        state.rm_calls.push(adapter_id.to_string());
        state.rm_results.pop_front().unwrap_or(Ok(()))
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        let mut state = self.inner.lock().unwrap();
        state.rmi_calls.push(image_ref.to_string());
        state.rmi_results.pop_front().unwrap_or(Ok(()))
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        let mut state = self.inner.lock().unwrap();
        state.wait_calls.push(adapter_id.to_string());
        state.wait_results.pop_front().unwrap_or(Ok(0))
    }
}

// ---------------------------------------------------------------------------
// Tests — install flow, single-adapter constraint, property tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterState, AdapterStateEvent};
    use crate::grpc::UpdateServiceImpl;
    use crate::state::StateManager;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    /// Helper: create service wiring for tests.
    fn make_service(
        mock: Arc<MockPodmanExecutor>,
    ) -> (
        UpdateServiceImpl<MockPodmanExecutor>,
        Arc<StateManager>,
        broadcast::Sender<AdapterStateEvent>,
    ) {
        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));
        let service = UpdateServiceImpl::new(state_mgr.clone(), mock, tx.clone());
        (service, state_mgr, tx)
    }

    // -- TS-07-1: InstallAdapter returns response immediately ---------------

    #[tokio::test]
    async fn test_install_response_immediate() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc123".to_string()));
        mock.set_run_result(Ok(()));
        let (service, _state_mgr, _tx) = make_service(mock);

        let resp = service
            .install_adapter(
                "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
                "sha256:abc123",
            )
            .await
            .unwrap();

        // job_id must be a valid UUID v4
        let parsed = uuid::Uuid::parse_str(&resp.job_id).expect("job_id should be valid UUID");
        assert_eq!(
            parsed.get_version(),
            Some(uuid::Version::Random),
            "job_id should be UUID v4"
        );
        assert_eq!(resp.adapter_id, "parkhaus-munich-v1.0.0");
        assert_eq!(resp.state, AdapterState::Downloading);
    }

    // -- TS-07-2: Podman pull executed on install ---------------------------

    #[tokio::test]
    async fn test_install_calls_podman_pull() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc123".to_string()));
        mock.set_run_result(Ok(()));
        let (service, _state_mgr, _tx) = make_service(mock.clone());

        let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
        service
            .install_adapter(image_ref, "sha256:abc123")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(mock.pull_calls(), vec![image_ref]);
    }

    // -- TS-07-3: Checksum verification after pull --------------------------

    #[tokio::test]
    async fn test_install_verifies_checksum() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc123".to_string()));
        mock.set_run_result(Ok(()));
        let (service, state_mgr, _tx) = make_service(mock.clone());

        let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
        service
            .install_adapter(image_ref, "sha256:abc123")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(mock.inspect_calls().len(), 1);
        let adapter = state_mgr.get_adapter("parkhaus-munich-v1.0.0");
        assert!(adapter.is_some());
        let state = adapter.unwrap().state;
        // After checksum match, adapter should be Installing or Running
        assert!(
            state == AdapterState::Installing || state == AdapterState::Running,
            "expected Installing or Running, got {state:?}"
        );
    }

    // -- TS-07-4: Container started with --network=host --------------------

    #[tokio::test]
    async fn test_install_runs_with_network_host() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc123".to_string()));
        mock.set_run_result(Ok(()));
        let (service, _state_mgr, _tx) = make_service(mock.clone());

        let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
        service
            .install_adapter(image_ref, "sha256:abc123")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(
            mock.run_calls(),
            vec![("parkhaus-munich-v1.0.0".to_string(), image_ref.to_string())]
        );
    }

    // -- TS-07-5: State transitions to RUNNING on success -------------------

    #[tokio::test]
    async fn test_install_reaches_running() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc123".to_string()));
        mock.set_run_result(Ok(()));
        // Container stays running (wait blocks / returns later)
        let (service, state_mgr, _tx) = make_service(mock);

        let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
        service
            .install_adapter(image_ref, "sha256:abc123")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = state_mgr
            .get_adapter("parkhaus-munich-v1.0.0")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Running);
    }

    // -- TS-07-E1: Empty image_ref returns INVALID_ARGUMENT -----------------

    #[tokio::test]
    async fn test_install_empty_image_ref() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (service, _state_mgr, _tx) = make_service(mock);

        let result = service.install_adapter("", "sha256:abc123").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, crate::grpc::ServiceError::InvalidArgument(_)),
            "expected InvalidArgument, got {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("image_ref is required"),
            "error message should mention image_ref: {msg}"
        );
    }

    // -- TS-07-E2: Empty checksum returns INVALID_ARGUMENT ------------------

    #[tokio::test]
    async fn test_install_empty_checksum() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (service, _state_mgr, _tx) = make_service(mock);

        let result = service
            .install_adapter("example.com/img:v1", "")
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, crate::grpc::ServiceError::InvalidArgument(_)),
            "expected InvalidArgument, got {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("checksum_sha256 is required"),
            "error message should mention checksum: {msg}"
        );
    }

    // -- TS-07-E3: Podman pull failure transitions to ERROR -----------------

    #[tokio::test]
    async fn test_pull_failure_error_state() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Err(PodmanError::new("connection refused")));
        let (service, state_mgr, _tx) = make_service(mock);

        service
            .install_adapter("bad-registry.com/img:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = state_mgr
            .get_adapter("img-v1")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
        assert!(
            adapter
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("connection refused"),
            "error message should contain podman stderr"
        );
    }

    // -- TS-07-E4: Checksum mismatch transitions to ERROR and removes image -

    #[tokio::test]
    async fn test_checksum_mismatch_error() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:different".to_string()));
        let (service, state_mgr, _tx) = make_service(mock.clone());

        let image_ref = "example.com/img:v1";
        service
            .install_adapter(image_ref, "sha256:expected")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = state_mgr
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
            mock.rmi_calls().contains(&image_ref.to_string()),
            "image should be removed after checksum mismatch"
        );
    }

    // -- TS-07-E5: Podman run failure transitions to ERROR ------------------

    #[tokio::test]
    async fn test_run_failure_error_state() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Err(PodmanError::new("container create failed")));
        let (service, state_mgr, _tx) = make_service(mock);

        service
            .install_adapter("example.com/img:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let adapter = state_mgr
            .get_adapter("img-v1")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // -- TS-07-7: Single adapter constraint stops running adapter -----------

    #[tokio::test]
    async fn test_single_adapter_stops_running() {
        let mock = Arc::new(MockPodmanExecutor::new());
        // Adapter A: pull, inspect, run all succeed
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:aaa".to_string()));
        mock.set_run_result(Ok(()));
        // Adapter B: pull, inspect, run all succeed; stop for A succeeds
        mock.set_stop_result(Ok(()));
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:bbb".to_string()));
        mock.set_run_result(Ok(()));

        let (service, state_mgr, _tx) = make_service(mock.clone());

        // Install adapter A
        let image_a = "example.com/adapter-a:v1";
        service
            .install_adapter(image_a, "sha256:aaa")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(
            state_mgr.get_adapter("adapter-a-v1").unwrap().state,
            AdapterState::Running
        );

        // Install adapter B (should stop A first)
        let image_b = "example.com/adapter-b:v1";
        service
            .install_adapter(image_b, "sha256:bbb")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert!(
            mock.stop_calls().contains(&"adapter-a-v1".to_string()),
            "adapter A should have been stopped"
        );
        assert_eq!(
            state_mgr.get_adapter("adapter-a-v1").unwrap().state,
            AdapterState::Stopped
        );
        assert_eq!(
            state_mgr.get_adapter("adapter-b-v1").unwrap().state,
            AdapterState::Running
        );
    }

    // -- TS-07-E6: Stop failure still proceeds with new install -------------

    #[tokio::test]
    async fn test_stop_failure_install_proceeds() {
        let mock = Arc::new(MockPodmanExecutor::new());
        // Adapter A: succeeds
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:aaa".to_string()));
        mock.set_run_result(Ok(()));
        // Stop A fails
        mock.set_stop_result(Err(PodmanError::new("timeout")));
        // Adapter B: succeeds
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:bbb".to_string()));
        mock.set_run_result(Ok(()));

        let (service, state_mgr, _tx) = make_service(mock);

        // Install adapter A
        service
            .install_adapter("example.com/adapter-a:v1", "sha256:aaa")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Install adapter B (stop A fails)
        service
            .install_adapter("example.com/adapter-b:v1", "sha256:bbb")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert_eq!(
            state_mgr.get_adapter("adapter-a-v1").unwrap().state,
            AdapterState::Error,
            "adapter A should be in Error state after failed stop"
        );
        assert_eq!(
            state_mgr.get_adapter("adapter-b-v1").unwrap().state,
            AdapterState::Running,
            "adapter B install should proceed despite failed stop of A"
        );
    }

    // -- TS-07-P2: Single adapter invariant property test -------------------

    #[test]
    #[ignore] // Run with --include-ignored
    fn proptest_single_adapter_invariant() {
        use proptest::prelude::*;

        let rt = tokio::runtime::Runtime::new().unwrap();

        proptest!(|(count in 1usize..5)| {
            rt.block_on(async {
                let mock = Arc::new(MockPodmanExecutor::new());
                for _ in 0..count {
                    mock.set_pull_result(Ok(()));
                    mock.set_inspect_result(Ok("sha256:ok".to_string()));
                    mock.set_run_result(Ok(()));
                    mock.set_stop_result(Ok(()));
                }
                let (tx, _rx) = broadcast::channel(64);
                let state_mgr = Arc::new(StateManager::new(tx.clone()));
                let service = UpdateServiceImpl::new(state_mgr.clone(), mock, tx);

                for i in 0..count {
                    let img = format!("registry.test/adapter-{}:v1", i);
                    let _ = service.install_adapter(&img, "sha256:ok").await;
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

                    let running_count = state_mgr
                        .list_adapters()
                        .iter()
                        .filter(|a| a.state == AdapterState::Running)
                        .count();
                    prop_assert!(
                        running_count <= 1,
                        "At most one adapter should be RUNNING, got {}", running_count
                    );
                }
                Ok::<(), proptest::test_runner::TestCaseError>(())
            })?;
        });
    }

    // -- TS-07-P5: Checksum verification soundness property test ------------

    #[test]
    #[ignore] // Run with --include-ignored
    fn proptest_checksum_verification_soundness() {
        use proptest::prelude::*;

        let rt = tokio::runtime::Runtime::new().unwrap();

        proptest!(|(
            digest in "sha256:[a-f0-9]{8}",
            checksum in "sha256:[a-f0-9]{8}",
        )| {
            if digest == checksum {
                return Ok(());
            }
            rt.block_on(async {
                let mock = Arc::new(MockPodmanExecutor::new());
                mock.set_pull_result(Ok(()));
                mock.set_inspect_result(Ok(digest.clone()));
                let (tx, _rx) = broadcast::channel(64);
                let state_mgr = Arc::new(StateManager::new(tx.clone()));
                let service = UpdateServiceImpl::new(state_mgr.clone(), mock.clone(), tx);

                let image_ref = "registry.test/adapter:v1";
                let _ = service.install_adapter(image_ref, &checksum).await;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                let adapter = state_mgr.get_adapter("adapter-v1");
                prop_assert!(adapter.is_some(), "adapter should exist");
                prop_assert_eq!(adapter.unwrap().state, AdapterState::Error);
                prop_assert!(
                    mock.rmi_calls().contains(&image_ref.to_string()),
                    "image should be removed after mismatch"
                );
                Ok::<(), proptest::test_runner::TestCaseError>(())
            })?;
        });
    }
}
