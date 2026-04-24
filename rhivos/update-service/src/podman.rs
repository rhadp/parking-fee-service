use std::fmt;

/// Error type for podman operations.
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

impl fmt::Display for PodmanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "podman error: {}", self.message)
    }
}

impl std::error::Error for PodmanError {}

/// Trait abstracting podman CLI operations for testability.
#[async_trait::async_trait]
pub trait PodmanExecutor: Send + Sync {
    /// Pull an OCI image: `podman pull <image_ref>`
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError>;

    /// Inspect the digest of a pulled image: `podman image inspect --format '{{.Digest}}' <image_ref>`
    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError>;

    /// Start a container: `podman run -d --name <adapter_id> --network=host <image_ref>`
    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError>;

    /// Stop a container: `podman stop <adapter_id>`
    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError>;

    /// Remove a container: `podman rm <adapter_id>`
    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError>;

    /// Remove an image: `podman rmi <image_ref>`
    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError>;

    /// Wait for a container to exit: `podman wait <adapter_id>`
    /// Returns the container exit code.
    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError>;
}

/// Real implementation using `tokio::process::Command` to shell out to podman.
pub struct RealPodmanExecutor;

impl RealPodmanExecutor {
    /// Run a podman command and return its output.
    async fn run_podman(&self, args: &[&str]) -> Result<std::process::Output, PodmanError> {
        tokio::process::Command::new("podman")
            .args(args)
            .output()
            .await
            .map_err(|e| PodmanError::new(format!("failed to execute podman: {e}")))
    }

    /// Check the exit status of a podman command output.
    fn check_status(output: &std::process::Output, operation: &str) -> Result<(), PodmanError> {
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(PodmanError::new(format!("{operation} failed: {stderr}")))
        }
    }
}

#[async_trait::async_trait]
impl PodmanExecutor for RealPodmanExecutor {
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
        let output = self.run_podman(&["pull", image_ref]).await?;
        Self::check_status(&output, "podman pull")
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        let output = self
            .run_podman(&["image", "inspect", "--format", "{{.Digest}}", image_ref])
            .await?;
        Self::check_status(&output, "podman image inspect")?;
        let digest = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(digest)
    }

    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
        let output = self
            .run_podman(&[
                "run",
                "-d",
                "--name",
                adapter_id,
                "--network=host",
                image_ref,
            ])
            .await?;
        Self::check_status(&output, "podman run")
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let output = self.run_podman(&["stop", adapter_id]).await?;
        Self::check_status(&output, "podman stop")
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        let output = self.run_podman(&["rm", adapter_id]).await?;
        Self::check_status(&output, "podman rm")
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        let output = self.run_podman(&["rmi", image_ref]).await?;
        Self::check_status(&output, "podman rmi")
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        let output = self.run_podman(&["wait", adapter_id]).await?;
        Self::check_status(&output, "podman wait")?;
        let code_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        code_str
            .parse::<i32>()
            .map_err(|e| PodmanError::new(format!("failed to parse exit code: {e}")))
    }
}

/// Mock implementation of PodmanExecutor for unit tests.
///
/// Records all calls and returns configurable results.
pub mod mock {
    use super::*;
    use std::sync::Mutex;

    /// A configurable mock that records calls and returns preset results.
    pub struct MockPodmanExecutor {
        pull_result: Mutex<Option<Result<(), PodmanError>>>,
        inspect_result: Mutex<Option<Result<String, PodmanError>>>,
        run_result: Mutex<Option<Result<(), PodmanError>>>,
        stop_result: Mutex<Option<Result<(), PodmanError>>>,
        rm_result: Mutex<Option<Result<(), PodmanError>>>,
        rmi_result: Mutex<Option<Result<(), PodmanError>>>,
        wait_result: Mutex<Option<Result<i32, PodmanError>>>,

        pub pull_calls: Mutex<Vec<String>>,
        pub inspect_calls: Mutex<Vec<String>>,
        pub run_calls: Mutex<Vec<(String, String)>>,
        pub stop_calls: Mutex<Vec<String>>,
        pub rm_calls: Mutex<Vec<String>>,
        pub rmi_calls: Mutex<Vec<String>>,
        pub wait_calls: Mutex<Vec<String>>,
    }

    impl Default for MockPodmanExecutor {
        fn default() -> Self {
            Self::new()
        }
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

        pub fn pull_calls(&self) -> Vec<String> {
            self.pull_calls.lock().unwrap().clone()
        }

        pub fn inspect_calls(&self) -> Vec<String> {
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
    }

    #[async_trait::async_trait]
    impl PodmanExecutor for MockPodmanExecutor {
        async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
            self.pull_calls
                .lock()
                .unwrap()
                .push(image_ref.to_string());
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
                .unwrap_or(Ok("sha256:default".to_string()))
        }

        async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
            self.run_calls
                .lock()
                .unwrap()
                .push((adapter_id.to_string(), image_ref.to_string()));
            self.run_result.lock().unwrap().clone().unwrap_or(Ok(()))
        }

        async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
            self.stop_calls
                .lock()
                .unwrap()
                .push(adapter_id.to_string());
            self.stop_result.lock().unwrap().clone().unwrap_or(Ok(()))
        }

        async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
            self.rm_calls
                .lock()
                .unwrap()
                .push(adapter_id.to_string());
            self.rm_result.lock().unwrap().clone().unwrap_or(Ok(()))
        }

        async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
            self.rmi_calls
                .lock()
                .unwrap()
                .push(image_ref.to_string());
            self.rmi_result.lock().unwrap().clone().unwrap_or(Ok(()))
        }

        async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
            self.wait_calls
                .lock()
                .unwrap()
                .push(adapter_id.to_string());
            self.wait_result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(Ok(0))
        }
    }
}
