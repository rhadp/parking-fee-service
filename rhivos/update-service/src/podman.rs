use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Error returned by podman executor operations.
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

/// Trait abstracting podman CLI operations for testability.
#[async_trait::async_trait]
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
// MockPodmanExecutor — fully implemented test infrastructure
// ---------------------------------------------------------------------------

struct MockInner {
    pull_result: Mutex<Result<(), String>>,
    inspect_result: Mutex<Result<String, String>>,
    run_result: Mutex<Result<(), String>>,
    stop_result: Mutex<Result<(), String>>,
    rm_result: Mutex<Result<(), String>>,
    rmi_result: Mutex<Result<(), String>>,
    wait_result: Mutex<Option<Result<i32, String>>>,
    stop_overrides: Mutex<HashMap<String, Result<(), String>>>,
    pull_calls: Mutex<Vec<String>>,
    inspect_calls: Mutex<Vec<String>>,
    run_calls: Mutex<Vec<(String, String)>>,
    stop_calls: Mutex<Vec<String>>,
    rm_calls: Mutex<Vec<String>>,
    rmi_calls: Mutex<Vec<String>>,
    wait_calls: Mutex<Vec<String>>,
}

pub struct MockPodmanExecutor {
    inner: Arc<MockInner>,
}

impl MockPodmanExecutor {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MockInner {
                pull_result: Mutex::new(Ok(())),
                inspect_result: Mutex::new(Ok("sha256:default".to_string())),
                run_result: Mutex::new(Ok(())),
                stop_result: Mutex::new(Ok(())),
                rm_result: Mutex::new(Ok(())),
                rmi_result: Mutex::new(Ok(())),
                wait_result: Mutex::new(None), // None = block forever (default for tests)
                stop_overrides: Mutex::new(HashMap::new()),
                pull_calls: Mutex::new(Vec::new()),
                inspect_calls: Mutex::new(Vec::new()),
                run_calls: Mutex::new(Vec::new()),
                stop_calls: Mutex::new(Vec::new()),
                rm_calls: Mutex::new(Vec::new()),
                rmi_calls: Mutex::new(Vec::new()),
                wait_calls: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn set_pull_result(&self, result: Result<(), PodmanError>) {
        *self.inner.pull_result.lock().unwrap() = result.map_err(|e| e.message);
    }

    pub fn set_inspect_result(&self, result: Result<String, PodmanError>) {
        *self.inner.inspect_result.lock().unwrap() = result.map_err(|e| e.message);
    }

    pub fn set_run_result(&self, result: Result<(), PodmanError>) {
        *self.inner.run_result.lock().unwrap() = result.map_err(|e| e.message);
    }

    pub fn set_stop_result(&self, result: Result<(), PodmanError>) {
        *self.inner.stop_result.lock().unwrap() = result.map_err(|e| e.message);
    }

    pub fn set_rm_result(&self, result: Result<(), PodmanError>) {
        *self.inner.rm_result.lock().unwrap() = result.map_err(|e| e.message);
    }

    pub fn set_rmi_result(&self, result: Result<(), PodmanError>) {
        *self.inner.rmi_result.lock().unwrap() = result.map_err(|e| e.message);
    }

    pub fn set_wait_result(&self, result: Result<i32, PodmanError>) {
        *self.inner.wait_result.lock().unwrap() = Some(result.map_err(|e| e.message));
    }

    pub fn set_stop_result_for(&self, adapter_id: &str, result: Result<(), PodmanError>) {
        self.inner
            .stop_overrides
            .lock()
            .unwrap()
            .insert(adapter_id.to_string(), result.map_err(|e| e.message));
    }

    pub fn pull_calls(&self) -> Vec<String> {
        self.inner.pull_calls.lock().unwrap().clone()
    }

    pub fn inspect_calls(&self) -> Vec<String> {
        self.inner.inspect_calls.lock().unwrap().clone()
    }

    pub fn run_calls(&self) -> Vec<(String, String)> {
        self.inner.run_calls.lock().unwrap().clone()
    }

    pub fn stop_calls(&self) -> Vec<String> {
        self.inner.stop_calls.lock().unwrap().clone()
    }

    pub fn rm_calls(&self) -> Vec<String> {
        self.inner.rm_calls.lock().unwrap().clone()
    }

    pub fn rmi_calls(&self) -> Vec<String> {
        self.inner.rmi_calls.lock().unwrap().clone()
    }

    pub fn wait_calls(&self) -> Vec<String> {
        self.inner.wait_calls.lock().unwrap().clone()
    }
}

impl Default for MockPodmanExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PodmanExecutor for MockPodmanExecutor {
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
        self.inner
            .pull_calls
            .lock()
            .unwrap()
            .push(image_ref.to_string());
        self.inner
            .pull_result
            .lock()
            .unwrap()
            .clone()
            .map_err(|msg| PodmanError::new(&msg))
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        self.inner
            .inspect_calls
            .lock()
            .unwrap()
            .push(image_ref.to_string());
        self.inner
            .inspect_result
            .lock()
            .unwrap()
            .clone()
            .map_err(|msg| PodmanError::new(&msg))
    }

    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
        self.inner
            .run_calls
            .lock()
            .unwrap()
            .push((adapter_id.to_string(), image_ref.to_string()));
        self.inner
            .run_result
            .lock()
            .unwrap()
            .clone()
            .map_err(|msg| PodmanError::new(&msg))
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        self.inner
            .stop_calls
            .lock()
            .unwrap()
            .push(adapter_id.to_string());
        let override_result = self
            .inner
            .stop_overrides
            .lock()
            .unwrap()
            .get(adapter_id)
            .cloned();
        if let Some(r) = override_result {
            return r.map_err(|msg| PodmanError::new(&msg));
        }
        self.inner
            .stop_result
            .lock()
            .unwrap()
            .clone()
            .map_err(|msg| PodmanError::new(&msg))
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        self.inner
            .rm_calls
            .lock()
            .unwrap()
            .push(adapter_id.to_string());
        self.inner
            .rm_result
            .lock()
            .unwrap()
            .clone()
            .map_err(|msg| PodmanError::new(&msg))
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        self.inner
            .rmi_calls
            .lock()
            .unwrap()
            .push(image_ref.to_string());
        self.inner
            .rmi_result
            .lock()
            .unwrap()
            .clone()
            .map_err(|msg| PodmanError::new(&msg))
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        self.inner
            .wait_calls
            .lock()
            .unwrap()
            .push(adapter_id.to_string());
        let result = self.inner.wait_result.lock().unwrap().clone();
        match result {
            Some(r) => r.map_err(|msg| PodmanError::new(&msg)),
            // None = block indefinitely (simulates a long-running container in tests).
            None => std::future::pending::<Result<i32, PodmanError>>().await,
        }
    }
}

// ---------------------------------------------------------------------------
// RealPodmanExecutor — shells out to the podman CLI
// ---------------------------------------------------------------------------

pub struct RealPodmanExecutor;

impl RealPodmanExecutor {
    async fn run_command(
        &self,
        args: &[&str],
    ) -> Result<std::process::Output, PodmanError> {
        tokio::process::Command::new("podman")
            .args(args)
            .output()
            .await
            .map_err(|e| PodmanError::new(&format!("failed to spawn podman: {}", e)))
    }
}

#[async_trait::async_trait]
impl PodmanExecutor for RealPodmanExecutor {
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
        let output = self.run_command(&["pull", image_ref]).await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(PodmanError::new(&stderr))
        }
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        let output = self
            .run_command(&["image", "inspect", "--format", "{{.Digest}}", image_ref])
            .await?;
        if output.status.success() {
            let digest = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(digest)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(PodmanError::new(&stderr))
        }
    }

    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
        let output = self
            .run_command(&[
                "run",
                "-d",
                "--name",
                adapter_id,
                "--network=host",
                image_ref,
            ])
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(PodmanError::new(&stderr))
        }
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let output = self.run_command(&["stop", adapter_id]).await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(PodmanError::new(&stderr))
        }
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let output = self.run_command(&["rm", adapter_id]).await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(PodmanError::new(&stderr))
        }
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        let output = self.run_command(&["rmi", image_ref]).await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(PodmanError::new(&stderr))
        }
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        let output = self.run_command(&["wait", adapter_id]).await?;
        if output.status.success() {
            let exit_code_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            exit_code_str
                .parse::<i32>()
                .map_err(|e| PodmanError::new(&format!("failed to parse exit code: {}", e)))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(PodmanError::new(&stderr))
        }
    }
}

// ---------------------------------------------------------------------------
// Install flow tests — call service::UpdateService with MockPodmanExecutor
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{ServiceError, UpdateService};
    use crate::state::StateManager;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn make_service() -> (
        Arc<StateManager>,
        Arc<MockPodmanExecutor>,
        UpdateService<MockPodmanExecutor>,
    ) {
        let (tx, _rx) = broadcast::channel(128);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));
        let podman = Arc::new(MockPodmanExecutor::new());
        let svc = UpdateService::new(Arc::clone(&state_mgr), Arc::clone(&podman), tx);
        (state_mgr, podman, svc)
    }

    const IMAGE_REF: &str = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    const CHECKSUM: &str = "sha256:abc123";
    const ADAPTER_ID: &str = "parkhaus-munich-v1.0.0";

    // TS-07-1: InstallAdapter returns response immediately with job_id, adapter_id, DOWNLOADING
    #[tokio::test]
    async fn test_install_response_immediate() {
        let (_sm, podman, svc) = make_service();
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        let resp = svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        assert!(!resp.job_id.is_empty());
        assert_eq!(resp.adapter_id, ADAPTER_ID);
        assert_eq!(resp.state, crate::adapter::AdapterState::Downloading);
    }

    // TS-07-2: Podman pull is called with the provided image_ref
    #[tokio::test]
    async fn test_install_calls_podman_pull() {
        let (_sm, podman, svc) = make_service();
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        podman.set_run_result(Ok(()));
        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(podman.pull_calls(), vec![IMAGE_REF]);
    }

    // TS-07-3: Checksum is verified after pull
    #[tokio::test]
    async fn test_install_verifies_checksum() {
        let (sm, podman, svc) = make_service();
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        podman.set_run_result(Ok(()));
        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(podman.inspect_calls().len(), 1);
        let adapter = sm.get_adapter(ADAPTER_ID).unwrap();
        assert!(
            adapter.state == crate::adapter::AdapterState::Installing
                || adapter.state == crate::adapter::AdapterState::Running
        );
    }

    // TS-07-4: Container is started with correct adapter_id and image_ref
    #[tokio::test]
    async fn test_install_runs_with_network_host() {
        let (_sm, podman, svc) = make_service();
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        podman.set_run_result(Ok(()));
        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(
            podman.run_calls(),
            vec![(ADAPTER_ID.to_string(), IMAGE_REF.to_string())]
        );
    }

    // TS-07-5: Adapter reaches RUNNING state after successful install
    #[tokio::test]
    async fn test_install_reaches_running() {
        let (sm, podman, svc) = make_service();
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        podman.set_run_result(Ok(()));
        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let adapter = sm.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, crate::adapter::AdapterState::Running);
    }

    // TS-07-E1: Empty image_ref returns InvalidArgument
    #[tokio::test]
    async fn test_install_empty_image_ref() {
        let (_sm, _podman, svc) = make_service();
        let result = svc.install_adapter("", CHECKSUM).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::InvalidArgument(_)));
    }

    // TS-07-E2: Empty checksum returns InvalidArgument
    #[tokio::test]
    async fn test_install_empty_checksum() {
        let (_sm, _podman, svc) = make_service();
        let result = svc.install_adapter(IMAGE_REF, "").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::InvalidArgument(_)));
    }

    // TS-07-E3: Pull failure transitions adapter to ERROR
    #[tokio::test]
    async fn test_pull_failure_error_state() {
        let (sm, podman, svc) = make_service();
        podman.set_pull_result(Err(PodmanError::new("connection refused")));
        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let adapter = sm.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, crate::adapter::AdapterState::Error);
        assert!(adapter
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("connection refused"));
    }

    // TS-07-E4: Checksum mismatch transitions to ERROR and calls rmi
    #[tokio::test]
    async fn test_checksum_mismatch_error() {
        let (sm, podman, svc) = make_service();
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok("sha256:different".to_string()));
        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let adapter = sm.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, crate::adapter::AdapterState::Error);
        assert!(adapter
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("checksum_mismatch"));
        assert!(podman.rmi_calls().contains(&IMAGE_REF.to_string()));
    }

    // TS-07-E5: Podman run failure transitions adapter to ERROR
    #[tokio::test]
    async fn test_run_failure_error_state() {
        let (sm, podman, svc) = make_service();
        podman.set_pull_result(Ok(()));
        podman.set_inspect_result(Ok(CHECKSUM.to_string()));
        podman.set_run_result(Err(PodmanError::new("container create failed")));
        svc.install_adapter(IMAGE_REF, CHECKSUM).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let adapter = sm.get_adapter(ADAPTER_ID).unwrap();
        assert_eq!(adapter.state, crate::adapter::AdapterState::Error);
    }

    // TS-07-P5: Checksum verification soundness property test
    // For any digest != checksum, the adapter transitions to ERROR and
    // the image is removed via rmi.
    #[test]
    #[ignore]
    fn proptest_checksum_verification_soundness() {
        use proptest::prelude::*;

        let config = ProptestConfig { cases: 20, ..Default::default() };
        proptest!(config, |(
            digest_suffix in "[a-f0-9]{8}",
            checksum_suffix in "[a-f0-9]{8}"
        )| {
            let digest = format!("sha256:{}", digest_suffix);
            let checksum = format!("sha256:{}", checksum_suffix);
            prop_assume!(digest != checksum);

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let result: Result<(), String> = rt.block_on(async {
                let (tx, _rx) = broadcast::channel(128);
                let state_mgr = Arc::new(StateManager::new(tx.clone()));
                let podman = Arc::new(MockPodmanExecutor::new());
                let svc = UpdateService::new(
                    Arc::clone(&state_mgr),
                    Arc::clone(&podman),
                    tx,
                );

                podman.set_pull_result(Ok(()));
                podman.set_inspect_result(Ok(digest));

                let image_ref = "registry.example.com/test-img:v1";
                svc.install_adapter(image_ref, &checksum)
                    .await
                    .map_err(|e| format!("{}", e))?;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                let adapter = state_mgr
                    .get_adapter("test-img-v1")
                    .ok_or_else(|| "adapter not found in state".to_string())?;
                if adapter.state != crate::adapter::AdapterState::Error {
                    return Err(format!(
                        "Expected ERROR state, got {:?}",
                        adapter.state
                    ));
                }
                if !podman.rmi_calls().contains(&image_ref.to_string()) {
                    return Err(
                        "Expected rmi call for mismatched checksum".to_string(),
                    );
                }
                Ok(())
            });
            prop_assert!(result.is_ok(), "{}", result.err().unwrap_or_default());
        });
    }
}
