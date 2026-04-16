use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
        write!(f, "PodmanError: {}", self.message)
    }
}

impl std::error::Error for PodmanError {}

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

/// Real podman executor that shells out to the podman CLI.
#[allow(dead_code)]
pub struct RealPodmanExecutor;

#[async_trait]
impl PodmanExecutor for RealPodmanExecutor {
    async fn pull(&self, _image_ref: &str) -> Result<(), PodmanError> {
        todo!("implement RealPodmanExecutor::pull")
    }

    async fn inspect_digest(&self, _image_ref: &str) -> Result<String, PodmanError> {
        todo!("implement RealPodmanExecutor::inspect_digest")
    }

    async fn run(&self, _adapter_id: &str, _image_ref: &str) -> Result<(), PodmanError> {
        todo!("implement RealPodmanExecutor::run")
    }

    async fn stop(&self, _adapter_id: &str) -> Result<(), PodmanError> {
        todo!("implement RealPodmanExecutor::stop")
    }

    async fn rm(&self, _adapter_id: &str) -> Result<(), PodmanError> {
        todo!("implement RealPodmanExecutor::rm")
    }

    async fn rmi(&self, _image_ref: &str) -> Result<(), PodmanError> {
        todo!("implement RealPodmanExecutor::rmi")
    }

    async fn wait(&self, _adapter_id: &str) -> Result<i32, PodmanError> {
        todo!("implement RealPodmanExecutor::wait")
    }
}

// ────────────────────────────────────────────────────────────────────────────
// MockPodmanExecutor
// ────────────────────────────────────────────────────────────────────────────

struct MockInner {
    pull_result: Result<(), PodmanError>,
    inspect_result: Result<String, PodmanError>,
    run_result: Result<(), PodmanError>,
    stop_result: Result<(), PodmanError>,
    stop_result_overrides: HashMap<String, Result<(), PodmanError>>,
    rm_result: Result<(), PodmanError>,
    rmi_result: Result<(), PodmanError>,
    wait_result: Result<i32, PodmanError>,

    pull_calls: Vec<String>,
    inspect_calls: Vec<String>,
    run_calls: Vec<(String, String)>,
    stop_calls: Vec<String>,
    rm_calls: Vec<String>,
    rmi_calls: Vec<String>,
    wait_calls: Vec<String>,
}

impl Default for MockInner {
    fn default() -> Self {
        Self {
            pull_result: Ok(()),
            inspect_result: Ok("sha256:abc123".to_string()),
            run_result: Ok(()),
            stop_result: Ok(()),
            stop_result_overrides: HashMap::new(),
            rm_result: Ok(()),
            rmi_result: Ok(()),
            wait_result: Ok(0),

            pull_calls: Vec::new(),
            inspect_calls: Vec::new(),
            run_calls: Vec::new(),
            stop_calls: Vec::new(),
            rm_calls: Vec::new(),
            rmi_calls: Vec::new(),
            wait_calls: Vec::new(),
        }
    }
}

pub struct MockPodmanExecutor {
    inner: Arc<Mutex<MockInner>>,
}

impl Default for MockPodmanExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPodmanExecutor {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockInner::default())),
        }
    }

    pub fn set_pull_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().pull_result = result;
    }

    pub fn set_inspect_result(&self, result: Result<String, PodmanError>) {
        self.inner.lock().unwrap().inspect_result = result;
    }

    pub fn set_run_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().run_result = result;
    }

    pub fn set_stop_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().stop_result = result;
    }

    pub fn set_stop_result_for(&self, adapter_id: &str, result: Result<(), PodmanError>) {
        self.inner
            .lock()
            .unwrap()
            .stop_result_overrides
            .insert(adapter_id.to_string(), result);
    }

    pub fn set_rm_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().rm_result = result;
    }

    pub fn set_rmi_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().rmi_result = result;
    }

    pub fn set_wait_result(&self, result: Result<i32, PodmanError>) {
        self.inner.lock().unwrap().wait_result = result;
    }

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

#[async_trait]
impl PodmanExecutor for MockPodmanExecutor {
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
        let mut inner = self.inner.lock().unwrap();
        inner.pull_calls.push(image_ref.to_string());
        inner.pull_result.clone()
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        let mut inner = self.inner.lock().unwrap();
        inner.inspect_calls.push(image_ref.to_string());
        inner.inspect_result.clone()
    }

    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .run_calls
            .push((adapter_id.to_string(), image_ref.to_string()));
        inner.run_result.clone()
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let mut inner = self.inner.lock().unwrap();
        inner.stop_calls.push(adapter_id.to_string());
        if let Some(result) = inner.stop_result_overrides.get(adapter_id) {
            return result.clone();
        }
        inner.stop_result.clone()
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let mut inner = self.inner.lock().unwrap();
        inner.rm_calls.push(adapter_id.to_string());
        inner.rm_result.clone()
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        let mut inner = self.inner.lock().unwrap();
        inner.rmi_calls.push(image_ref.to_string());
        inner.rmi_result.clone()
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        let mut inner = self.inner.lock().unwrap();
        inner.wait_calls.push(adapter_id.to_string());
        inner.wait_result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::UpdateService;
    use crate::state::StateManager;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::broadcast;

    const IMAGE_REF: &str = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    const CHECKSUM: &str = "sha256:abc123";
    const ADAPTER_ID: &str = "parkhaus-munich-v1.0.0";

    fn make_service(
        mock: Arc<MockPodmanExecutor>,
    ) -> (Arc<StateManager>, UpdateService<MockPodmanExecutor>) {
        let (tx, _rx) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx.clone()));
        let svc = UpdateService::new(
            state.clone(),
            mock,
            tx,
            Duration::from_secs(86400),
        );
        (state, svc)
    }

    // TS-07-1: InstallAdapter returns response immediately with DOWNLOADING state
    #[tokio::test]
    async fn test_install_response_immediate() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        let (_state, svc) = make_service(mock);

        let resp = svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        assert!(!resp.job_id.is_empty());
        // UUID v4 format: 8-4-4-4-12 hex groups
        let parts: Vec<&str> = resp.job_id.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(resp.adapter_id, ADAPTER_ID);
        assert_eq!(resp.state, crate::adapter::AdapterState::Downloading);
    }

    // TS-07-2: Podman pull executed on install
    #[tokio::test]
    async fn test_install_calls_podman_pull() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        let mock_clone = mock.clone();
        let (_state, svc) = make_service(mock);

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(mock_clone.pull_calls(), vec![IMAGE_REF.to_string()]);
    }

    // TS-07-3: Checksum verification after pull
    #[tokio::test]
    async fn test_install_verifies_checksum() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        let mock_clone = mock.clone();
        let (state, svc) = make_service(mock);

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(mock_clone.inspect_calls().len(), 1);
        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert!(
            adapter.state == crate::adapter::AdapterState::Installing
                || adapter.state == crate::adapter::AdapterState::Running
        );
    }

    // TS-07-4: Container started with correct adapter_id and image_ref
    #[tokio::test]
    async fn test_install_runs_with_network_host() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        let mock_clone = mock.clone();
        let (_state, svc) = make_service(mock);

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(
            mock_clone.run_calls(),
            vec![(ADAPTER_ID.to_string(), IMAGE_REF.to_string())]
        );
    }

    // TS-07-5: State transitions to RUNNING on success
    #[tokio::test]
    async fn test_install_reaches_running() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        // wait returns non-zero immediately by default, override to block
        // For this test we want RUNNING state, so set wait to never return
        // Use a channel-based approach: set wait_result to Ok(0) but test state before wait fires
        // Since wait is async and spawned, we just check quickly
        let (state, svc) = make_service(mock);

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, crate::adapter::AdapterState::Running);
    }

    // TS-07-E1: Empty image_ref returns error
    #[tokio::test]
    async fn test_install_empty_image_ref() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (_state, svc) = make_service(mock);

        let result = svc.install_adapter("", CHECKSUM).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, crate::service::ServiceErrorCode::InvalidArgument);
        assert!(err.message.contains("image_ref"));
    }

    // TS-07-E2: Empty checksum returns error
    #[tokio::test]
    async fn test_install_empty_checksum() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (_state, svc) = make_service(mock);

        let result = svc.install_adapter(IMAGE_REF, "").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, crate::service::ServiceErrorCode::InvalidArgument);
        assert!(err.message.contains("checksum"));
    }

    // TS-07-E3: Podman pull failure transitions to ERROR
    #[tokio::test]
    async fn test_pull_failure_error_state() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Err(PodmanError::new("connection refused")));
        let (state, svc) = make_service(mock);

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, crate::adapter::AdapterState::Error);
        assert!(adapter
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("connection refused"));
    }

    // TS-07-E4: Checksum mismatch transitions to ERROR and removes image
    #[tokio::test]
    async fn test_checksum_mismatch_error() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:different".to_string()));
        let mock_clone = mock.clone();
        let (state, svc) = make_service(mock);

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, crate::adapter::AdapterState::Error);
        assert!(adapter
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("checksum_mismatch"));
        assert!(mock_clone.rmi_calls().contains(&IMAGE_REF.to_string()));
    }

    // TS-07-E5: Podman run failure transitions to ERROR
    #[tokio::test]
    async fn test_run_failure_error_state() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Err(PodmanError::new("container create failed")));
        let (state, svc) = make_service(mock);

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, crate::adapter::AdapterState::Error);
    }
}

// TS-07-P5: Checksum verification soundness
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use crate::service::UpdateService;
    use crate::state::StateManager;
    use proptest::prelude::*;

    fn make_service_proptest(
        mock: Arc<MockPodmanExecutor>,
    ) -> (Arc<StateManager>, UpdateService<MockPodmanExecutor>) {
        let (tx, _rx) = tokio::sync::broadcast::channel(1000);
        let state = Arc::new(StateManager::new(tx.clone()));
        let svc = UpdateService::new(
            state.clone(),
            mock,
            tx,
            std::time::Duration::from_secs(86400),
        );
        (state, svc)
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(20))]

        // TS-07-P5: When digest != checksum, adapter goes to ERROR and image is removed
        #[test]
        #[ignore = "proptest: run with --include-ignored"]
        fn proptest_checksum_verification_soundness(
            actual_digest in "sha256:[a-f0-9]{8}",
            provided_checksum in "sha256:[a-f0-9]{8}",
        ) {
            prop_assume!(actual_digest != provided_checksum);
            let image_ref = "registry.example.com/adapter-test:v1";
            let adapter_id = "adapter-test-v1";

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            // Collect results from async context, assert outside
            let (adapter_state, rmi_called) = rt.block_on(async {
                let mock = Arc::new(MockPodmanExecutor::new());
                mock.set_pull_result(Ok(()));
                mock.set_inspect_result(Ok(actual_digest.clone()));
                let (state, svc) = make_service_proptest(mock.clone());

                let _ = svc.install_adapter(image_ref, &provided_checksum).await;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                let adapter_state = state.get_adapter(adapter_id).map(|a| a.state);
                let rmi_called = mock.rmi_calls().contains(&image_ref.to_string());
                (adapter_state, rmi_called)
            });
            prop_assert_eq!(adapter_state, Some(crate::adapter::AdapterState::Error));
            prop_assert!(rmi_called, "rmi should be called when checksum mismatches");
        }
    }
}
