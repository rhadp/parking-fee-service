//! Container runtime abstraction for managing adapter containers.
//!
//! Implements 04-REQ-4.E3 (container start failure -> ERROR state).
//!
//! The production implementation invokes `podman` via `std::process::Command`.
//! A mock implementation is provided for in-process testing.

/// Trait abstracting container runtime operations so tests can inject a mock.
#[async_trait::async_trait]
pub trait ContainerRuntime: Send + Sync + 'static {
    /// Create and start a container from the given image.
    ///
    /// Returns the container ID on success, or an error string on failure.
    async fn start_container(
        &self,
        adapter_id: &str,
        image_ref: &str,
    ) -> Result<String, String>;

    /// Stop a running container.
    async fn stop_container(&self, container_id: &str) -> Result<(), String>;

    /// Remove a container and its resources.
    async fn remove_container(&self, container_id: &str) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// Production implementation using podman CLI
// ---------------------------------------------------------------------------

/// Production container runtime using podman CLI.
#[derive(Default)]
pub struct PodmanRuntime;

impl PodmanRuntime {
    pub fn new() -> Self {
        PodmanRuntime
    }
}

#[async_trait::async_trait]
impl ContainerRuntime for PodmanRuntime {
    async fn start_container(
        &self,
        adapter_id: &str,
        image_ref: &str,
    ) -> Result<String, String> {
        // Create the container
        let create_output = tokio::process::Command::new("podman")
            .args(["create", "--name", adapter_id, image_ref])
            .output()
            .await
            .map_err(|e| format!("failed to execute podman create: {}", e))?;

        if !create_output.status.success() {
            let stderr = String::from_utf8_lossy(&create_output.stderr);
            return Err(format!("container create failed: {}", stderr.trim()));
        }

        let container_id = String::from_utf8_lossy(&create_output.stdout)
            .trim()
            .to_string();

        // Start the container
        let start_output = tokio::process::Command::new("podman")
            .args(["start", &container_id])
            .output()
            .await
            .map_err(|e| format!("failed to execute podman start: {}", e))?;

        if !start_output.status.success() {
            let stderr = String::from_utf8_lossy(&start_output.stderr);
            return Err(format!("container start failed: {}", stderr.trim()));
        }

        Ok(container_id)
    }

    async fn stop_container(&self, container_id: &str) -> Result<(), String> {
        let output = tokio::process::Command::new("podman")
            .args(["stop", container_id])
            .output()
            .await
            .map_err(|e| format!("failed to execute podman stop: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("container stop failed: {}", stderr.trim()));
        }

        Ok(())
    }

    async fn remove_container(&self, container_id: &str) -> Result<(), String> {
        let output = tokio::process::Command::new("podman")
            .args(["rm", "-f", container_id])
            .output()
            .await
            .map_err(|e| format!("failed to execute podman rm: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("container remove failed: {}", stderr.trim()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Mock implementation for testing
// ---------------------------------------------------------------------------

/// A mock container runtime for in-process testing.
///
/// Can be configured to succeed or fail container operations.
#[derive(Default)]
pub struct MockContainerRuntime {
    /// If true, `start_container` will fail with an error.
    pub fail_start: bool,
}

impl MockContainerRuntime {
    /// Create a mock runtime that succeeds on all operations.
    pub fn new() -> Self {
        MockContainerRuntime { fail_start: false }
    }

    /// Create a mock runtime that fails on container start.
    pub fn failing() -> Self {
        MockContainerRuntime { fail_start: true }
    }
}

#[async_trait::async_trait]
impl ContainerRuntime for MockContainerRuntime {
    async fn start_container(
        &self,
        adapter_id: &str,
        _image_ref: &str,
    ) -> Result<String, String> {
        if self.fail_start {
            return Err("container start failed: mock failure".to_string());
        }
        Ok(format!("container-{}", adapter_id))
    }

    async fn stop_container(&self, _container_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn remove_container(&self, _container_id: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_runtime_start_success() {
        let runtime = MockContainerRuntime::new();
        let result = runtime
            .start_container("test-adapter", "test:v1")
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test-adapter"));
    }

    #[tokio::test]
    async fn test_mock_runtime_start_failure() {
        let runtime = MockContainerRuntime::failing();
        let result = runtime
            .start_container("test-adapter", "test:v1")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mock failure"));
    }

    #[tokio::test]
    async fn test_mock_runtime_stop_and_remove() {
        let runtime = MockContainerRuntime::new();
        assert!(runtime.stop_container("container-1").await.is_ok());
        assert!(runtime.remove_container("container-1").await.is_ok());
    }
}
