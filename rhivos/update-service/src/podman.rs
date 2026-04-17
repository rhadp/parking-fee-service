//! Podman executor trait, real CLI implementation, and mock.
//!
//! The `PodmanExecutor` trait abstracts all container operations so that unit
//! tests can use `MockPodmanExecutor` while production code uses
//! `RealPodmanExecutor`.

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

// ── Real executor ─────────────────────────────────────────────────────────────

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
            .map_err(|e| PodmanError::new(e.to_string()))?;
        if !output.status.success() {
            return Err(PodmanError::new(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(())
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["image", "inspect", "--format", "{{.Digest}}", image_ref])
            .output()
            .await
            .map_err(|e| PodmanError::new(e.to_string()))?;
        if !output.status.success() {
            return Err(PodmanError::new(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
            .map_err(|e| PodmanError::new(e.to_string()))?;
        if !output.status.success() {
            return Err(PodmanError::new(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(())
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["stop", adapter_id])
            .output()
            .await
            .map_err(|e| PodmanError::new(e.to_string()))?;
        if !output.status.success() {
            return Err(PodmanError::new(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(())
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["rm", adapter_id])
            .output()
            .await
            .map_err(|e| PodmanError::new(e.to_string()))?;
        if !output.status.success() {
            return Err(PodmanError::new(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(())
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["rmi", image_ref])
            .output()
            .await
            .map_err(|e| PodmanError::new(e.to_string()))?;
        if !output.status.success() {
            return Err(PodmanError::new(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(())
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["wait", adapter_id])
            .output()
            .await
            .map_err(|e| PodmanError::new(e.to_string()))?;
        if !output.status.success() {
            return Err(PodmanError::new(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        let exit_code_str = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        exit_code_str
            .parse::<i32>()
            .map_err(|e| PodmanError::new(format!("failed to parse exit code: {e}")))
    }
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
    /// `None` = block forever (pending); `Some` = return the value immediately.
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
/// When `wait_result` is left unset (default), `wait()` blocks indefinitely
/// (simulating a long-running container). Set it explicitly with
/// `set_wait_result` to make the container "exit" with a specific code or
/// error.
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

    /// Set the result for `wait`. Pass `Ok(exit_code)` or `Err(err)`.
    /// If not set, `wait` blocks indefinitely (simulating a running container).
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
        // Record the call while holding the lock, then release it before awaiting.
        let result = {
            let mut g = self.inner.lock().unwrap();
            g.wait_calls.push(adapter_id.to_string());
            g.wait_result.clone()
        };
        match result {
            Some(Ok(code)) => Ok(code),
            Some(Err(e)) => Err(e),
            // None: block indefinitely — simulates a long-running container.
            None => futures::future::pending().await,
        }
    }
}

// ── Service implementation ────────────────────────────────────────────────────

/// Combined service struct that orchestrates install, remove, query, and watch
/// operations using a state manager and a podman executor.
#[allow(dead_code)]
pub struct UpdateServiceImpl<P: PodmanExecutor> {
    pub state: Arc<crate::state::StateManager>,
    pub podman: Arc<P>,
    pub broadcaster: tokio::sync::broadcast::Sender<crate::adapter::AdapterStateEvent>,
}

impl<P: PodmanExecutor + Send + Sync + 'static> UpdateServiceImpl<P> {
    pub fn new(
        state: Arc<crate::state::StateManager>,
        podman: Arc<P>,
        broadcaster: tokio::sync::broadcast::Sender<crate::adapter::AdapterStateEvent>,
    ) -> Self {
        Self {
            state,
            podman,
            broadcaster,
        }
    }

    /// Validate inputs, derive adapter_id, enforce the single-adapter
    /// constraint, create a state entry, spawn the background install task, and
    /// return immediately with job_id + DOWNLOADING state.
    pub async fn install_adapter(
        &self,
        image_ref: &str,
        checksum_sha256: &str,
    ) -> Result<crate::adapter::InstallResponse, tonic::Status> {
        // Validate inputs.
        if image_ref.is_empty() {
            return Err(tonic::Status::invalid_argument("image_ref is required"));
        }
        if checksum_sha256.is_empty() {
            return Err(tonic::Status::invalid_argument(
                "checksum_sha256 is required",
            ));
        }

        let adapter_id = crate::adapter::derive_adapter_id(image_ref);

        // Single adapter constraint: stop any currently RUNNING adapter first.
        if let Some(running) = self.state.get_running_adapter() {
            if running.adapter_id != adapter_id {
                match self.podman.stop(&running.adapter_id).await {
                    Ok(()) => {
                        let _ = self.state.transition(
                            &running.adapter_id,
                            crate::adapter::AdapterState::Stopped,
                            None,
                        );
                    }
                    Err(e) => {
                        self.state
                            .force_error(&running.adapter_id, e.message.clone());
                    }
                }
            }
        }

        // Generate a UUID v4 job ID.
        let job_id = uuid::Uuid::new_v4().to_string();

        // Create the adapter entry and transition it to DOWNLOADING.
        let entry = crate::adapter::AdapterEntry {
            adapter_id: adapter_id.clone(),
            image_ref: image_ref.to_string(),
            checksum_sha256: checksum_sha256.to_string(),
            state: crate::adapter::AdapterState::Unknown,
            job_id: job_id.clone(),
            stopped_at: None,
            error_message: None,
        };
        self.state.create_adapter(entry);
        let _ = self
            .state
            .transition(&adapter_id, crate::adapter::AdapterState::Downloading, None);

        // Spawn the background install task.
        let podman = Arc::clone(&self.podman);
        let state = Arc::clone(&self.state);
        let image_ref_owned = image_ref.to_string();
        let adapter_id_owned = adapter_id.clone();
        let checksum_owned = checksum_sha256.to_string();

        tokio::spawn(async move {
            // 1. Pull the image.
            if let Err(e) = podman.pull(&image_ref_owned).await {
                state.force_error(&adapter_id_owned, e.message);
                return;
            }

            // 2. Inspect the digest and verify the checksum.
            let digest = match podman.inspect_digest(&image_ref_owned).await {
                Ok(d) => d,
                Err(e) => {
                    state.force_error(&adapter_id_owned, e.message);
                    return;
                }
            };

            if digest != checksum_owned {
                // Checksum mismatch: remove the pulled image and error.
                let _ = podman.rmi(&image_ref_owned).await;
                state.force_error(&adapter_id_owned, "checksum_mismatch".to_string());
                return;
            }

            // 3. Transition to INSTALLING.
            let _ = state.transition(
                &adapter_id_owned,
                crate::adapter::AdapterState::Installing,
                None,
            );

            // 4. Start the container.
            if let Err(e) = podman.run(&adapter_id_owned, &image_ref_owned).await {
                state.force_error(&adapter_id_owned, e.message);
                return;
            }

            // 5. Transition to RUNNING.
            let _ = state.transition(
                &adapter_id_owned,
                crate::adapter::AdapterState::Running,
                None,
            );

            // 6. Monitor container exit via `podman wait`.
            //    Exit code 0 → STOPPED; non-zero / error → ERROR.
            match podman.wait(&adapter_id_owned).await {
                Ok(0) => {
                    let _ = state.transition(
                        &adapter_id_owned,
                        crate::adapter::AdapterState::Stopped,
                        None,
                    );
                }
                Ok(code) => {
                    state.force_error(
                        &adapter_id_owned,
                        format!("container exited with code {code}"),
                    );
                }
                Err(e) => {
                    state.force_error(&adapter_id_owned, e.message);
                }
            }
        });

        Ok(crate::adapter::InstallResponse {
            job_id,
            adapter_id,
            state: crate::adapter::AdapterState::Downloading,
        })
    }

    /// Stop the container (if running), remove the container, remove the image,
    /// and delete the adapter from state.
    pub async fn remove_adapter(&self, adapter_id: &str) -> Result<(), tonic::Status> {
        let entry = self
            .state
            .get_adapter(adapter_id)
            .ok_or_else(|| tonic::Status::not_found("adapter not found"))?;

        // Stop if running.
        if entry.state == crate::adapter::AdapterState::Running {
            if let Err(e) = self.podman.stop(adapter_id).await {
                self.state.force_error(adapter_id, e.message.clone());
                return Err(tonic::Status::internal(e.message));
            }
        }

        // Remove container.
        if let Err(e) = self.podman.rm(adapter_id).await {
            self.state.force_error(adapter_id, e.message.clone());
            return Err(tonic::Status::internal(e.message));
        }

        // Remove image.
        if let Err(e) = self.podman.rmi(&entry.image_ref).await {
            self.state.force_error(adapter_id, e.message.clone());
            return Err(tonic::Status::internal(e.message));
        }

        // Remove from state.
        let _ = self.state.remove_adapter(adapter_id);
        Ok(())
    }

    /// Return the adapter entry or NOT_FOUND.
    #[allow(clippy::result_large_err)]
    pub fn get_adapter_status(
        &self,
        adapter_id: &str,
    ) -> Result<crate::adapter::AdapterEntry, tonic::Status> {
        self.state
            .get_adapter(adapter_id)
            .ok_or_else(|| tonic::Status::not_found("adapter not found"))
    }

    /// Delegate to the state manager.
    pub fn list_adapters(&self) -> Vec<crate::adapter::AdapterEntry> {
        self.state.list_adapters()
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

    /// Helper to create an AdapterEntry directly in RUNNING state.
    fn running_entry(adapter_id: &str, image_ref: &str, checksum: &str) -> crate::adapter::AdapterEntry {
        crate::adapter::AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: checksum.to_string(),
            state: AdapterState::Running,
            job_id: "job-direct".to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    // ── TS-07-1: install returns DOWNLOADING immediately ──────────────────

    /// TS-07-1: InstallAdapter returns job_id (UUID v4), adapter_id, DOWNLOADING state
    #[tokio::test]
    async fn test_install_response_immediate() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        // wait not set → blocks forever; adapter stays RUNNING
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
        // wait not set → blocks forever
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
        // wait not set → blocks; adapter stays in RUNNING after successful install
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
        // wait not set → blocks
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
        // wait not set → blocks indefinitely, keeping adapter in RUNNING
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

    /// TS-07-7: second install stops running adapter first.
    ///
    /// Adapter A is placed directly into RUNNING state to avoid race conditions
    /// with the background install task. Adapter B's install must stop A first.
    #[tokio::test]
    async fn test_single_adapter_stops_running() {
        const IMAGE_B: &str = "reg.io/adapter-b:v1";
        const CHECKSUM_B: &str = "sha256:bbb";
        const ADAPTER_B: &str = "adapter-b-v1";

        let (tx, _rx) = broadcast::channel(64);
        let state = Arc::new(StateManager::new(tx.clone()));
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_run_result(Ok(()));
        // wait not set → blocks; adapters stay in RUNNING once started

        // Place adapter A directly into RUNNING state (no background task for A).
        state.create_adapter(running_entry(ADAPTER_ID, IMAGE_REF, CHECKSUM));

        // Configure inspect result for adapter B.
        mock.set_inspect_result(Ok(CHECKSUM_B.to_string()));

        let svc = UpdateServiceImpl::new(Arc::clone(&state), Arc::clone(&mock), tx);

        // Install adapter B — must stop A first.
        svc.install_adapter(IMAGE_B, CHECKSUM_B).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        assert!(
            mock.stop_calls().contains(&ADAPTER_ID.to_string()),
            "stop should have been called for adapter A"
        );
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

    /// TS-07-E6: stop failure -> old adapter ERROR, new install proceeds.
    ///
    /// Adapter A is placed directly into RUNNING state. Stop fails for A.
    /// Install of B must still proceed and reach RUNNING.
    #[tokio::test]
    async fn test_stop_failure_install_proceeds() {
        const IMAGE_B: &str = "reg.io/adapter-b:v1";
        const CHECKSUM_B: &str = "sha256:bbb";
        const ADAPTER_B: &str = "adapter-b-v1";

        let (tx, _rx) = broadcast::channel(64);
        let state = Arc::new(StateManager::new(tx.clone()));
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_run_result(Ok(()));
        // wait not set → blocks; B stays in RUNNING

        // Place adapter A directly into RUNNING state.
        state.create_adapter(running_entry(ADAPTER_ID, IMAGE_REF, CHECKSUM));

        // Configure stop to fail for adapter A.
        mock.set_stop_result_for(ADAPTER_ID, Err(PodmanError::new("timeout")));
        mock.set_inspect_result(Ok(CHECKSUM_B.to_string()));

        let svc = UpdateServiceImpl::new(Arc::clone(&state), Arc::clone(&mock), tx);

        // Install adapter B.
        svc.install_adapter(IMAGE_B, CHECKSUM_B).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // A should be in ERROR (stop failed).
        assert_eq!(
            state.get_adapter(ADAPTER_ID).unwrap().state,
            AdapterState::Error
        );
        // B should be RUNNING (install proceeded despite A's stop failure).
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

    /// TS-07-E11: remove_adapter with rm failure -> INTERNAL.
    ///
    /// Adapter is placed directly in RUNNING state to avoid timing issues.
    #[tokio::test]
    async fn test_removal_failure_internal() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_rm_result(Err(PodmanError::new("container in use")));
        let (svc, state, _rx) = make_service(Arc::clone(&mock));

        // Place adapter directly in RUNNING state.
        state.create_adapter(running_entry(ADAPTER_ID, IMAGE_REF, CHECKSUM));

        let result = svc.remove_adapter(ADAPTER_ID).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::Internal);
        let adapter = state.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
    }

    // ── Property test scaffolds ───────────────────────────────────────────

    /// TS-07-P2: single adapter invariant (property test).
    ///
    /// At most one adapter is in RUNNING state at any point. Tested at the
    /// state manager level (simulating the constraint that install_adapter
    /// enforces by stopping the running adapter before starting a new one).
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_single_adapter_invariant() {
        use crate::adapter::{AdapterEntry, AdapterState};
        use proptest::prelude::*;

        proptest!(|(n in 2usize..=5)| {
            let (tx, _rx) = broadcast::channel::<crate::adapter::AdapterStateEvent>(128);
            let state = Arc::new(StateManager::new(tx));

            let mut prev_id: Option<String> = None;

            for i in 0..n {
                let adapter_id = format!("adapter-{i}-v1");
                let image_ref = format!("reg.io/adapter-{i}:v1");

                // Simulate single-adapter constraint: stop previous running adapter.
                if let Some(ref prev) = prev_id {
                    let _ = state.transition(prev, AdapterState::Stopped, None);
                }

                // Create new adapter directly in RUNNING state.
                state.create_adapter(AdapterEntry {
                    adapter_id: adapter_id.clone(),
                    image_ref,
                    checksum_sha256: "sha256:abc".to_string(),
                    state: AdapterState::Running,
                    job_id: "job".to_string(),
                    stopped_at: None,
                    error_message: None,
                });

                // Verify at most one RUNNING at all times.
                let running = state
                    .list_adapters()
                    .iter()
                    .filter(|a| a.state == AdapterState::Running)
                    .count();
                prop_assert!(running <= 1, "invariant violated: {} adapters running", running);

                prev_id = Some(adapter_id);
            }
        });
    }

    /// TS-07-P5: checksum verification soundness (property test).
    ///
    /// When the pulled image digest differs from the provided checksum, the
    /// adapter transitions to ERROR and rmi is called.
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_checksum_verification_soundness() {
        use proptest::prelude::*;

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(
            actual_hex in "[a-f0-9]{64}",
            provided_hex in "[a-f0-9]{64}"
        )| {
            // Only test when digest != checksum.
            prop_assume!(actual_hex != provided_hex);

            let image_ref = "reg.io/test-img:v1";
            let adapter_id = "test-img-v1";
            let actual_digest = format!("sha256:{actual_hex}");
            let provided = format!("sha256:{provided_hex}");

            let (state_val, rmi_called) = rt.block_on(async {
                let (tx, _rx) = broadcast::channel::<crate::adapter::AdapterStateEvent>(64);
                let state = Arc::new(StateManager::new(tx.clone()));
                let mock = Arc::new(MockPodmanExecutor::new());
                mock.set_pull_result(Ok(()));
                mock.set_inspect_result(Ok(actual_digest.clone()));

                let svc = UpdateServiceImpl::new(Arc::clone(&state), Arc::clone(&mock), tx);
                let _ = svc.install_adapter(image_ref, &provided).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let st = state.get_adapter(adapter_id).map(|a| a.state);
                let rmi = mock.rmi_calls().contains(&image_ref.to_string());
                (st, rmi)
            });

            prop_assert_eq!(
                state_val,
                Some(AdapterState::Error),
                "adapter should be ERROR on checksum mismatch"
            );
            prop_assert!(rmi_called, "rmi should be called on checksum mismatch");
        });
    }

    /// TS-07-P6: offload timing correctness (property test scaffold)
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_offload_timing_correctness() {
        // Implemented in task group 4.
        todo!()
    }
}
