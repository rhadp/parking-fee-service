//! Podman executor trait and mock implementation.
//!
//! The `PodmanExecutor` trait abstracts all container operations so that unit
//! tests can use `MockPodmanExecutor` while production code uses
//! `RealPodmanExecutor` (implemented in task group 3).

#![allow(dead_code)]

use async_trait::async_trait;
use std::sync::{Arc, Mutex};

// ── Error type ───────────────────────────────────────────────────────────────

/// Error returned by podman operations.
#[derive(Debug, Clone)]
pub struct PodmanError {
    pub message: String,
}

impl PodmanError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PodmanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "podman error: {}", self.message)
    }
}

impl std::error::Error for PodmanError {}

// ── Executor trait ───────────────────────────────────────────────────────────

/// Abstraction over podman CLI operations.
#[async_trait]
pub trait PodmanExecutor: Send + Sync {
    /// Pull an OCI image.
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError>;

    /// Inspect the manifest digest of a pulled image.
    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError>;

    /// Start a detached container with `--network=host`.
    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError>;

    /// Stop a running container.
    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError>;

    /// Remove a stopped container.
    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError>;

    /// Remove an image.
    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError>;

    /// Block until a container exits. Returns the exit code.
    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError>;
}

// ── Mock executor ────────────────────────────────────────────────────────────

/// Per-adapter configurable stop result.
#[derive(Default)]
struct MockInner {
    pull_result: Option<Result<(), PodmanError>>,
    inspect_result: Option<Result<String, PodmanError>>,
    run_result: Option<Result<(), PodmanError>>,
    stop_result: Option<Result<(), PodmanError>>,
    /// Per-adapter-id override for stop.
    stop_result_for: std::collections::HashMap<String, Result<(), PodmanError>>,
    rm_result: Option<Result<(), PodmanError>>,
    rmi_result: Option<Result<(), PodmanError>>,
    wait_result: Option<Result<i32, PodmanError>>,

    pull_calls: Vec<String>,
    inspect_calls: Vec<String>,
    run_calls: Vec<(String, String)>,
    stop_calls: Vec<String>,
    rm_calls: Vec<String>,
    rmi_calls: Vec<String>,
    wait_calls: Vec<String>,
}

/// A configurable, call-tracking mock of `PodmanExecutor`.
///
/// All `set_*_result` methods must be called before the async task that drives
/// the install flow runs (i.e., before or immediately after calling
/// `install_adapter`).
#[derive(Clone, Default)]
pub struct MockPodmanExecutor {
    inner: Arc<Mutex<MockInner>>,
}

impl MockPodmanExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Configuration helpers ─────────────────────────────────────────────

    pub fn set_pull_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().pull_result = Some(result);
    }

    pub fn set_inspect_result(&self, result: Result<String, PodmanError>) {
        self.inner.lock().unwrap().inspect_result = Some(result);
    }

    pub fn set_run_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().run_result = Some(result);
    }

    pub fn set_stop_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().stop_result = Some(result);
    }

    /// Configure a per-adapter-id stop result (overrides the global one).
    pub fn set_stop_result_for(&self, adapter_id: impl Into<String>, result: Result<(), PodmanError>) {
        self.inner
            .lock()
            .unwrap()
            .stop_result_for
            .insert(adapter_id.into(), result);
    }

    pub fn set_rm_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().rm_result = Some(result);
    }

    pub fn set_rmi_result(&self, result: Result<(), PodmanError>) {
        self.inner.lock().unwrap().rmi_result = Some(result);
    }

    pub fn set_wait_result(&self, result: Result<i32, PodmanError>) {
        self.inner.lock().unwrap().wait_result = Some(result);
    }

    // ── Call inspection helpers ───────────────────────────────────────────

    pub fn pull_calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().pull_calls.clone()
    }

    pub fn inspect_digest_calls(&self) -> Vec<String> {
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
        let mut g = self.inner.lock().unwrap();
        g.pull_calls.push(image_ref.to_string());
        match g.pull_result.clone() {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => Err(e),
            None => Ok(()),
        }
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        let mut g = self.inner.lock().unwrap();
        g.inspect_calls.push(image_ref.to_string());
        match g.inspect_result.clone() {
            Some(Ok(digest)) => Ok(digest),
            Some(Err(e)) => Err(e),
            None => Ok(String::new()),
        }
    }

    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
        let mut g = self.inner.lock().unwrap();
        g.run_calls.push((adapter_id.to_string(), image_ref.to_string()));
        match g.run_result.clone() {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => Err(e),
            None => Ok(()),
        }
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let mut g = self.inner.lock().unwrap();
        g.stop_calls.push(adapter_id.to_string());
        // Per-adapter override takes precedence.
        if let Some(r) = g.stop_result_for.get(adapter_id).cloned() {
            return match r {
                Ok(()) => Ok(()),
                Err(e) => Err(e),
            };
        }
        match g.stop_result.clone() {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => Err(e),
            None => Ok(()),
        }
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let mut g = self.inner.lock().unwrap();
        g.rm_calls.push(adapter_id.to_string());
        match g.rm_result.clone() {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => Err(e),
            None => Ok(()),
        }
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        let mut g = self.inner.lock().unwrap();
        g.rmi_calls.push(image_ref.to_string());
        match g.rmi_result.clone() {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => Err(e),
            None => Ok(()),
        }
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        let mut g = self.inner.lock().unwrap();
        g.wait_calls.push(adapter_id.to_string());
        match g.wait_result.clone() {
            Some(Ok(code)) => Ok(code),
            Some(Err(e)) => Err(e),
            // Default: block forever (never resolves) — callers must set a result.
            None => Err(PodmanError::new("wait result not configured")),
        }
    }
}

// ── Service stub (used by tests; full impl in task group 3) ──────────────────

/// Combined service struct used by unit tests.
///
/// The constructor and all methods panic with `todo!()` until task group 3
/// provides the real implementation.
#[allow(dead_code)]
pub struct UpdateServiceImpl<P: PodmanExecutor> {
    pub state: Arc<crate::state::StateManager>,
    pub podman: Arc<P>,
    pub broadcaster: tokio::sync::broadcast::Sender<crate::adapter::AdapterStateEvent>,
}

impl<P: PodmanExecutor + Send + Sync + 'static> UpdateServiceImpl<P> {
    pub fn new(
        _state: Arc<crate::state::StateManager>,
        _podman: Arc<P>,
        _broadcaster: tokio::sync::broadcast::Sender<crate::adapter::AdapterStateEvent>,
    ) -> Self {
        todo!()
    }

    /// Validate inputs, derive adapter_id, create state entry, spawn background
    /// install task, and return immediately with job_id + DOWNLOADING state.
    pub async fn install_adapter(
        &self,
        _image_ref: &str,
        _checksum_sha256: &str,
    ) -> Result<crate::adapter::InstallResponse, tonic::Status> {
        todo!()
    }

    /// Stop + rm + rmi the adapter, then remove it from state.
    pub async fn remove_adapter(&self, _adapter_id: &str) -> Result<(), tonic::Status> {
        todo!()
    }

    /// Return the adapter entry or NOT_FOUND.
    #[allow(clippy::result_large_err)]
    pub fn get_adapter_status(
        &self,
        _adapter_id: &str,
    ) -> Result<crate::adapter::AdapterEntry, tonic::Status> {
        todo!()
    }

    /// Delegate to the state manager.
    pub fn list_adapters(&self) -> Vec<crate::adapter::AdapterEntry> {
        todo!()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::AdapterState;
    use crate::state::StateManager;
    use tokio::sync::broadcast;

    const IMAGE_REF: &str = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    const CHECKSUM: &str = "sha256:abc123";
    const ADAPTER_ID: &str = "parkhaus-munich-v1.0.0";

    fn make_service(
        mock: Arc<MockPodmanExecutor>,
    ) -> (
        UpdateServiceImpl<MockPodmanExecutor>,
        Arc<StateManager>,
        tokio::sync::broadcast::Receiver<crate::adapter::AdapterStateEvent>,
    ) {
        let (tx, rx) = broadcast::channel(64);
        let state = Arc::new(StateManager::new(tx.clone()));
        let svc = UpdateServiceImpl::new(Arc::clone(&state), mock, tx);
        (svc, state, rx)
    }

    // ── TS-07-1: install returns DOWNLOADING immediately ──────────────────

    /// TS-07-1: InstallAdapter returns job_id (UUID v4), adapter_id, DOWNLOADING state
    #[tokio::test]
    async fn test_install_response_immediate() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(PodmanError::new("block")));
        let (svc, _state, _rx) = make_service(mock);

        let resp = svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        assert!(!resp.job_id.is_empty(), "job_id must be non-empty");
        assert_eq!(resp.adapter_id, ADAPTER_ID);
        assert_eq!(resp.state, AdapterState::Downloading);
    }

    // ── TS-07-2: install calls podman pull ────────────────────────────────

    /// TS-07-2: install calls podman pull with image_ref
    #[tokio::test]
    async fn test_install_calls_podman_pull() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(PodmanError::new("block")));
        let (svc, _state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_eq!(mock.pull_calls(), vec![IMAGE_REF.to_string()]);
    }

    // ── TS-07-3: checksum verification after pull ─────────────────────────

    /// TS-07-3: after pull, inspect_digest is called
    #[tokio::test]
    async fn test_install_verifies_checksum() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(PodmanError::new("block")));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert!(!mock.inspect_digest_calls().is_empty());
        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert!(
            adapter.state == AdapterState::Installing || adapter.state == AdapterState::Running,
            "state should be Installing or Running after checksum match"
        );
    }

    // ── TS-07-4: run called with correct args ─────────────────────────────

    /// TS-07-4: on checksum match, run is called with adapter_id and image_ref
    #[tokio::test]
    async fn test_install_runs_with_network_host() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(PodmanError::new("block")));
        let (svc, _state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_eq!(
            mock.run_calls(),
            vec![(ADAPTER_ID.to_string(), IMAGE_REF.to_string())]
        );
    }

    // ── TS-07-5: state reaches RUNNING ────────────────────────────────────

    /// TS-07-5: after successful run, state is RUNNING
    #[tokio::test]
    async fn test_install_reaches_running() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(PodmanError::new("block")));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, AdapterState::Running);
    }

    // ── TS-07-E1: empty image_ref ─────────────────────────────────────────

    /// TS-07-E1: empty image_ref returns INVALID_ARGUMENT
    #[tokio::test]
    async fn test_install_empty_image_ref() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (svc, _state, _rx) = make_service(mock);

        let result = svc.install_adapter("", CHECKSUM).await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("image_ref is required"));
    }

    // ── TS-07-E2: empty checksum ──────────────────────────────────────────

    /// TS-07-E2: empty checksum_sha256 returns INVALID_ARGUMENT
    #[tokio::test]
    async fn test_install_empty_checksum() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (svc, _state, _rx) = make_service(mock);

        let result = svc.install_adapter(IMAGE_REF, "").await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("checksum_sha256 is required"));
    }

    // ── TS-07-E3: pull failure ─────────────────────────────────────────────

    /// TS-07-E3: pull failure -> adapter ERROR
    #[tokio::test]
    async fn test_pull_failure_error_state() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Err(PodmanError::new("connection refused")));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
        assert!(adapter
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("connection refused"));
    }

    // ── TS-07-E4: checksum mismatch ───────────────────────────────────────

    /// TS-07-E4: checksum mismatch -> adapter ERROR + rmi called
    #[tokio::test]
    async fn test_checksum_mismatch_error() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:different".to_string()));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
        assert!(adapter
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("checksum_mismatch"));
        assert!(mock.rmi_calls().contains(&IMAGE_REF.to_string()));
    }

    // ── TS-07-E5: run failure ─────────────────────────────────────────────

    /// TS-07-E5: run failure -> adapter ERROR
    #[tokio::test]
    async fn test_run_failure_error_state() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Err(PodmanError::new("container create failed")));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // ── TS-07-7: single adapter constraint ───────────────────────────────

    /// TS-07-7: second install stops running adapter first
    #[tokio::test]
    async fn test_single_adapter_stops_running() {
        const IMAGE_B: &str = "reg.io/adapter-b:v1";
        const CHECKSUM_B: &str = "sha256:bbb";
        const ADAPTER_B: &str = "adapter-b-v1";

        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(PodmanError::new("block")));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        // Install adapter A
        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        assert_eq!(
            state.get_adapter(ADAPTER_ID).unwrap().state,
            AdapterState::Running
        );

        // Configure mock for adapter B
        mock.set_inspect_result(Ok(CHECKSUM_B.to_string()));

        // Install adapter B (should stop A first)
        svc.install_adapter(IMAGE_B, CHECKSUM_B).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        assert!(mock.stop_calls().contains(&ADAPTER_ID.to_string()));
        assert_eq!(
            state.get_adapter(ADAPTER_ID).unwrap().state,
            AdapterState::Stopped
        );
        assert_eq!(
            state.get_adapter(ADAPTER_B).unwrap().state,
            AdapterState::Running
        );
    }

    // ── TS-07-E6: stop failure, install proceeds ──────────────────────────

    /// TS-07-E6: stop failure -> old adapter ERROR, new install proceeds
    #[tokio::test]
    async fn test_stop_failure_install_proceeds() {
        const IMAGE_B: &str = "reg.io/adapter-b:v1";
        const CHECKSUM_B: &str = "sha256:bbb";
        const ADAPTER_B: &str = "adapter-b-v1";

        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(PodmanError::new("block")));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        // Install adapter A
        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Configure stop to fail for adapter A
        mock.set_stop_result_for(ADAPTER_ID, Err(PodmanError::new("timeout")));
        mock.set_inspect_result(Ok(CHECKSUM_B.to_string()));

        // Install adapter B
        svc.install_adapter(IMAGE_B, CHECKSUM_B).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        assert_eq!(
            state.get_adapter(ADAPTER_ID).unwrap().state,
            AdapterState::Error
        );
        assert_eq!(
            state.get_adapter(ADAPTER_B).unwrap().state,
            AdapterState::Running
        );
    }

    // ── TS-07-13: offload after timeout ───────────────────────────────────

    /// TS-07-13: offload after timeout
    #[tokio::test]
    async fn test_offload_after_timeout() {
        // Full implementation in task group 4.
        todo!()
    }

    // ── TS-07-E12: offload failure -> ERROR ───────────────────────────────

    /// TS-07-E12: offload failure -> adapter ERROR
    #[tokio::test]
    async fn test_offload_failure_error() {
        // Full implementation in task group 4.
        todo!()
    }

    // ── TS-07-15: container exit non-zero ─────────────────────────────────

    /// TS-07-15: container exit non-zero -> ERROR
    #[tokio::test]
    async fn test_container_exit_nonzero_error() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Ok(1)); // non-zero exit
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // ── TS-07-16: container exit 0 -> STOPPED ────────────────────────────

    /// TS-07-16: container exit 0 -> STOPPED
    #[tokio::test]
    async fn test_container_exit_zero_stopped() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Ok(0)); // clean exit
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, AdapterState::Stopped);
    }

    // ── TS-07-E16: podman wait failure -> ERROR ───────────────────────────

    /// TS-07-E16: podman wait failure -> ERROR
    #[tokio::test]
    async fn test_podman_wait_failure_error() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(PodmanError::new("connection lost")));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // ── TS-07-E11: removal failure -> INTERNAL ────────────────────────────

    /// TS-07-E11: remove_adapter with rm failure -> INTERNAL
    #[tokio::test]
    async fn test_removal_failure_internal() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        mock.set_wait_result(Err(PodmanError::new("block")));
        mock.set_rm_result(Err(PodmanError::new("container in use")));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let result = svc.remove_adapter(ADAPTER_ID).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::Internal);
        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // ── Property test scaffolds ───────────────────────────────────────────

    /// TS-07-P2: single adapter invariant (property test scaffold)
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_single_adapter_invariant() {
        // Implemented in task group 3.
        todo!()
    }

    /// TS-07-P5: checksum verification soundness (property test scaffold)
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_checksum_verification_soundness() {
        // Implemented in task group 3.
        todo!()
    }

    /// TS-07-P6: offload timing correctness (property test scaffold)
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_offload_timing_correctness() {
        // Implemented in task group 4.
        todo!()
    }
}
