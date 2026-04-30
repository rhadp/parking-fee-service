use async_trait::async_trait;

/// Error type for podman operations.
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

/// Real podman executor that shells out to the podman CLI via
/// `tokio::process::Command`.
pub struct RealPodmanExecutor;

impl RealPodmanExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RealPodmanExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PodmanExecutor for RealPodmanExecutor {
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(["pull", image_ref])
            .output()
            .await
            .map_err(|e| PodmanError::new(&format!("failed to execute podman pull: {e}")))?;

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
            .map_err(|e| {
                PodmanError::new(&format!("failed to execute podman image inspect: {e}"))
            })?;

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
            .map_err(|e| PodmanError::new(&format!("failed to execute podman run: {e}")))?;

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
            .map_err(|e| PodmanError::new(&format!("failed to execute podman stop: {e}")))?;

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
            .map_err(|e| PodmanError::new(&format!("failed to execute podman rm: {e}")))?;

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
            .map_err(|e| PodmanError::new(&format!("failed to execute podman rmi: {e}")))?;

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
            .map_err(|e| PodmanError::new(&format!("failed to execute podman wait: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::new(stderr.trim()));
        }

        let exit_code_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        exit_code_str
            .parse::<i32>()
            .map_err(|e| PodmanError::new(&format!("failed to parse exit code: {e}")))
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::sync::Mutex;

    /// Mock podman executor for unit tests.
    ///
    /// Uses `Mutex` for interior mutability so it is `Send + Sync` and
    /// compatible with multi-threaded tokio tests.
    ///
    /// **wait() behaviour:** By default, `wait()` blocks indefinitely
    /// (never returns) so that the container monitor does not interfere
    /// with install-flow assertions. Call `set_wait_immediate(true)` to
    /// make `wait()` return its configured result without blocking —
    /// use this in tests that directly exercise `monitor_container`.
    /// Call `complete_wait()` to release a single blocked `wait()` call
    /// when you want to test the monitor transition after install.
    pub struct MockPodmanExecutor {
        pull_result: Mutex<Option<Result<(), PodmanError>>>,
        inspect_result: Mutex<Option<Result<String, PodmanError>>>,
        run_result: Mutex<Option<Result<(), PodmanError>>>,
        stop_result: Mutex<Option<Result<(), PodmanError>>>,
        rm_result: Mutex<Option<Result<(), PodmanError>>>,
        rmi_result: Mutex<Option<Result<(), PodmanError>>>,
        wait_result: Mutex<Option<Result<i32, PodmanError>>>,
        pull_calls: Mutex<Vec<String>>,
        inspect_calls: Mutex<Vec<String>>,
        run_calls: Mutex<Vec<(String, String)>>,
        stop_calls: Mutex<Vec<String>>,
        rm_calls: Mutex<Vec<String>>,
        rmi_calls: Mutex<Vec<String>>,
        wait_calls: Mutex<Vec<String>>,
        // Per-adapter stop results for multi-adapter tests.
        stop_results_by_id: Mutex<std::collections::HashMap<String, Result<(), PodmanError>>>,
        /// When true, wait() returns immediately with configured result.
        /// When false (default), wait() blocks until complete_wait() is called.
        wait_immediate: Mutex<bool>,
        /// Notify used to release blocked wait() calls.
        wait_notify: tokio::sync::Notify,
    }

    impl MockPodmanExecutor {
        pub fn new() -> Self {
            Self {
                pull_result: Mutex::new(None),
                inspect_result: Mutex::new(None),
                run_result: Mutex::new(None),
                stop_result: Mutex::new(None),
                rm_result: Mutex::new(None),
                rmi_result: Mutex::new(None),
                wait_result: Mutex::new(None),
                pull_calls: Mutex::new(Vec::new()),
                inspect_calls: Mutex::new(Vec::new()),
                run_calls: Mutex::new(Vec::new()),
                stop_calls: Mutex::new(Vec::new()),
                rm_calls: Mutex::new(Vec::new()),
                rmi_calls: Mutex::new(Vec::new()),
                wait_calls: Mutex::new(Vec::new()),
                stop_results_by_id: Mutex::new(std::collections::HashMap::new()),
                wait_immediate: Mutex::new(false),
                wait_notify: tokio::sync::Notify::new(),
            }
        }

        pub fn set_pull_result(&self, result: Result<(), PodmanError>) {
            *self.pull_result.lock().unwrap() = Some(result);
        }

        pub fn set_inspect_result(&self, result: Result<String, PodmanError>) {
            *self.inspect_result.lock().unwrap() = Some(result);
        }

        pub fn set_run_result(&self, result: Result<(), PodmanError>) {
            *self.run_result.lock().unwrap() = Some(result);
        }

        pub fn set_stop_result(&self, result: Result<(), PodmanError>) {
            *self.stop_result.lock().unwrap() = Some(result);
        }

        pub fn set_rm_result(&self, result: Result<(), PodmanError>) {
            *self.rm_result.lock().unwrap() = Some(result);
        }

        pub fn set_rmi_result(&self, result: Result<(), PodmanError>) {
            *self.rmi_result.lock().unwrap() = Some(result);
        }

        pub fn set_wait_result(&self, result: Result<i32, PodmanError>) {
            *self.wait_result.lock().unwrap() = Some(result);
        }

        /// Configure wait() to return immediately without blocking.
        /// Use this in tests that directly exercise monitor_container.
        pub fn set_wait_immediate(&self, immediate: bool) {
            *self.wait_immediate.lock().unwrap() = immediate;
        }

        /// Release all currently blocked wait() calls so they return
        /// their configured result. Use this to simulate container exit
        /// after the install flow has settled.
        pub fn complete_wait(&self) {
            self.wait_notify.notify_waiters();
        }

        pub fn set_stop_result_for(&self, adapter_id: &str, result: Result<(), PodmanError>) {
            self.stop_results_by_id
                .lock()
                .unwrap()
                .insert(adapter_id.to_string(), result);
        }

        pub fn pull_calls(&self) -> Vec<String> {
            self.pull_calls.lock().unwrap().clone()
        }

        pub fn inspect_digest_calls(&self) -> Vec<String> {
            self.inspect_calls.lock().unwrap().clone()
        }

        pub fn run_calls(&self) -> Vec<(String, String)> {
            self.run_calls.lock().unwrap().clone()
        }

        pub fn stop_calls(&self) -> Vec<String> {
            self.stop_calls.lock().unwrap().clone()
        }

        pub fn rm_calls(&self) -> Vec<String> {
            self.rm_calls.lock().unwrap().clone()
        }

        pub fn rmi_calls(&self) -> Vec<String> {
            self.rmi_calls.lock().unwrap().clone()
        }

        pub fn wait_calls(&self) -> Vec<String> {
            self.wait_calls.lock().unwrap().clone()
        }
    }

    impl Default for MockPodmanExecutor {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl PodmanExecutor for MockPodmanExecutor {
        async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
            self.pull_calls.lock().unwrap().push(image_ref.to_string());
            self.pull_result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(Ok(()))
        }

        async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
            self.inspect_calls
                .lock()
                .unwrap()
                .push(image_ref.to_string());
            self.inspect_result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| Ok("sha256:default".to_string()))
        }

        async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
            self.run_calls
                .lock()
                .unwrap()
                .push((adapter_id.to_string(), image_ref.to_string()));
            self.run_result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(Ok(()))
        }

        async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
            self.stop_calls
                .lock()
                .unwrap()
                .push(adapter_id.to_string());
            // Check per-adapter results first.
            if let Some(result) = self
                .stop_results_by_id
                .lock()
                .unwrap()
                .get(adapter_id)
            {
                return result.clone();
            }
            self.stop_result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(Ok(()))
        }

        async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
            self.rm_calls
                .lock()
                .unwrap()
                .push(adapter_id.to_string());
            self.rm_result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(Ok(()))
        }

        async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
            self.rmi_calls
                .lock()
                .unwrap()
                .push(image_ref.to_string());
            self.rmi_result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(Ok(()))
        }

        async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
            self.wait_calls
                .lock()
                .unwrap()
                .push(adapter_id.to_string());
            // Block until the test signals completion, unless immediate
            // mode is enabled (used by direct monitor_container tests).
            let immediate = *self.wait_immediate.lock().unwrap();
            if !immediate {
                self.wait_notify.notified().await;
            }
            self.wait_result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(Ok(0))
        }
    }
}
