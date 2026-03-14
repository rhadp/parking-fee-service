use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// Errors returned by container runtime operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerError {
    PullFailed(String),
    InspectFailed(String),
    RunFailed(String),
    StopFailed(String),
    RemoveFailed(String),
    RemoveImageFailed(String),
}

impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerError::PullFailed(m) => write!(f, "pull failed: {}", m),
            ContainerError::InspectFailed(m) => write!(f, "inspect failed: {}", m),
            ContainerError::RunFailed(m) => write!(f, "run failed: {}", m),
            ContainerError::StopFailed(m) => write!(f, "stop failed: {}", m),
            ContainerError::RemoveFailed(m) => write!(f, "remove failed: {}", m),
            ContainerError::RemoveImageFailed(m) => write!(f, "remove image failed: {}", m),
        }
    }
}

impl std::error::Error for ContainerError {}

/// Trait abstracting the podman container runtime for testability.
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Pull the OCI image from the registry.
    async fn pull(&self, image_ref: &str) -> Result<(), ContainerError>;

    /// Inspect the pulled image and return its OCI manifest digest.
    async fn inspect_digest(&self, image_ref: &str) -> Result<String, ContainerError>;

    /// Run the container with `--network=host` and return the container ID.
    async fn run(&self, image_ref: &str, adapter_id: &str) -> Result<String, ContainerError>;

    /// Stop a running container.
    async fn stop(&self, container_id: &str) -> Result<(), ContainerError>;

    /// Remove a stopped container.
    async fn remove(&self, container_id: &str) -> Result<(), ContainerError>;

    /// Remove an image from local storage.
    async fn remove_image(&self, image_ref: &str) -> Result<(), ContainerError>;
}

// ---------------------------------------------------------------------------
// Mock implementation used in unit tests
// ---------------------------------------------------------------------------

/// Recorded call to the mock runtime.
#[derive(Debug, Clone)]
pub enum MockCall {
    Pull(String),
    InspectDigest(String),
    Run { image_ref: String, adapter_id: String },
    Stop(String),
    Remove(String),
    RemoveImage(String),
}

/// Configurable mock container runtime for unit testing.
#[derive(Clone)]
pub struct MockContainerRuntime {
    inner: Arc<Mutex<MockState>>,
}

struct MockState {
    /// Digest returned by inspect_digest.
    digest: String,
    /// If true, pull() returns an error.
    pull_error: bool,
    /// If true, inspect_digest() returns an error.
    inspect_error: bool,
    /// If true, run() returns an error.
    run_error: bool,
    /// If true, stop() returns an error.
    stop_error: bool,
    /// If true, remove() returns an error.
    remove_error: bool,
    /// If true, remove_image() returns an error.
    remove_image_error: bool,
    /// Recorded calls for assertions.
    calls: Vec<MockCall>,
}

impl MockContainerRuntime {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockState {
                digest: "sha256:default_digest".to_string(),
                pull_error: false,
                inspect_error: false,
                run_error: false,
                stop_error: false,
                remove_error: false,
                remove_image_error: false,
                calls: Vec::new(),
            })),
        }
    }

    /// Set the digest returned by inspect_digest.
    pub fn with_digest(self, digest: &str) -> Self {
        self.inner.lock().unwrap().digest = digest.to_string();
        self
    }

    pub fn set_pull_error(&self, val: bool) {
        self.inner.lock().unwrap().pull_error = val;
    }

    pub fn set_inspect_error(&self, val: bool) {
        self.inner.lock().unwrap().inspect_error = val;
    }

    pub fn set_run_error(&self, val: bool) {
        self.inner.lock().unwrap().run_error = val;
    }

    pub fn set_stop_error(&self, val: bool) {
        self.inner.lock().unwrap().stop_error = val;
    }

    pub fn set_remove_error(&self, val: bool) {
        self.inner.lock().unwrap().remove_error = val;
    }

    pub fn set_remove_image_error(&self, val: bool) {
        self.inner.lock().unwrap().remove_image_error = val;
    }

    /// Return all recorded calls.
    pub fn calls(&self) -> Vec<MockCall> {
        self.inner.lock().unwrap().calls.clone()
    }

    pub fn was_pull_called(&self) -> bool {
        self.calls().iter().any(|c| matches!(c, MockCall::Pull(_)))
    }

    pub fn was_run_called(&self) -> bool {
        self.calls().iter().any(|c| matches!(c, MockCall::Run { .. }))
    }

    pub fn was_stop_called(&self) -> bool {
        self.calls().iter().any(|c| matches!(c, MockCall::Stop(_)))
    }

    pub fn was_remove_called(&self) -> bool {
        self.calls().iter().any(|c| matches!(c, MockCall::Remove(_)))
    }

    pub fn was_remove_image_called(&self) -> bool {
        self.calls()
            .iter()
            .any(|c| matches!(c, MockCall::RemoveImage(_)))
    }
}

impl Default for MockContainerRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContainerRuntime for MockContainerRuntime {
    async fn pull(&self, image_ref: &str) -> Result<(), ContainerError> {
        let mut s = self.inner.lock().unwrap();
        s.calls.push(MockCall::Pull(image_ref.to_string()));
        if s.pull_error {
            return Err(ContainerError::PullFailed("mock pull error".to_string()));
        }
        Ok(())
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, ContainerError> {
        let mut s = self.inner.lock().unwrap();
        s.calls.push(MockCall::InspectDigest(image_ref.to_string()));
        if s.inspect_error {
            return Err(ContainerError::InspectFailed(
                "mock inspect error".to_string(),
            ));
        }
        Ok(s.digest.clone())
    }

    async fn run(&self, image_ref: &str, adapter_id: &str) -> Result<String, ContainerError> {
        let mut s = self.inner.lock().unwrap();
        s.calls.push(MockCall::Run {
            image_ref: image_ref.to_string(),
            adapter_id: adapter_id.to_string(),
        });
        if s.run_error {
            return Err(ContainerError::RunFailed("mock run error".to_string()));
        }
        Ok(format!("mock-container-{}", adapter_id))
    }

    async fn stop(&self, container_id: &str) -> Result<(), ContainerError> {
        let mut s = self.inner.lock().unwrap();
        s.calls.push(MockCall::Stop(container_id.to_string()));
        if s.stop_error {
            return Err(ContainerError::StopFailed("mock stop error".to_string()));
        }
        Ok(())
    }

    async fn remove(&self, container_id: &str) -> Result<(), ContainerError> {
        let mut s = self.inner.lock().unwrap();
        s.calls.push(MockCall::Remove(container_id.to_string()));
        if s.remove_error {
            return Err(ContainerError::RemoveFailed(
                "mock remove error".to_string(),
            ));
        }
        Ok(())
    }

    async fn remove_image(&self, image_ref: &str) -> Result<(), ContainerError> {
        let mut s = self.inner.lock().unwrap();
        s.calls.push(MockCall::RemoveImage(image_ref.to_string()));
        if s.remove_image_error {
            return Err(ContainerError::RemoveImageFailed(
                "mock remove image error".to_string(),
            ));
        }
        Ok(())
    }
}

/// Podman-backed container runtime (production implementation).
pub struct PodmanRuntime {
    pub storage_path: String,
}

#[async_trait]
impl ContainerRuntime for PodmanRuntime {
    /// Pull an OCI image using `podman pull`.
    async fn pull(&self, image_ref: &str) -> Result<(), ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args(["pull", image_ref])
            .output()
            .await
            .map_err(|e| ContainerError::PullFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ContainerError::PullFailed(stderr));
        }
        Ok(())
    }

    /// Retrieve the OCI manifest digest using `podman image inspect`.
    async fn inspect_digest(&self, image_ref: &str) -> Result<String, ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args([
                "image",
                "inspect",
                "--format",
                "{{.Digest}}",
                image_ref,
            ])
            .output()
            .await
            .map_err(|e| ContainerError::InspectFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ContainerError::InspectFailed(stderr));
        }

        let digest = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if digest.is_empty() {
            return Err(ContainerError::InspectFailed(
                "empty digest returned by podman image inspect".to_string(),
            ));
        }
        Ok(digest)
    }

    /// Run the container in detached mode with `--network=host`.
    ///
    /// Returns the container ID printed by `podman run`.
    async fn run(&self, image_ref: &str, adapter_id: &str) -> Result<String, ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args([
                "run",
                "--detach",
                "--network=host",
                &format!("--name={}", adapter_id),
                image_ref,
            ])
            .output()
            .await
            .map_err(|e| ContainerError::RunFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ContainerError::RunFailed(stderr));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(container_id)
    }

    /// Stop a running container with `podman stop`.
    async fn stop(&self, container_id: &str) -> Result<(), ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args(["stop", container_id])
            .output()
            .await
            .map_err(|e| ContainerError::StopFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ContainerError::StopFailed(stderr));
        }
        Ok(())
    }

    /// Remove a stopped container with `podman rm`.
    async fn remove(&self, container_id: &str) -> Result<(), ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args(["rm", container_id])
            .output()
            .await
            .map_err(|e| ContainerError::RemoveFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ContainerError::RemoveFailed(stderr));
        }
        Ok(())
    }

    /// Remove an image from local storage with `podman rmi`.
    async fn remove_image(&self, image_ref: &str) -> Result<(), ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args(["rmi", image_ref])
            .output()
            .await
            .map_err(|e| ContainerError::RemoveImageFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ContainerError::RemoveImageFailed(stderr));
        }
        Ok(())
    }
}
