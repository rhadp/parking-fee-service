use async_trait::async_trait;
use thiserror::Error;
use tracing::{debug, warn};

#[cfg(test)]
use mockall::automock;

/// Errors from container runtime operations.
#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("failed to run container: {0}")]
    RunFailed(String),

    #[error("failed to stop container: {0}")]
    StopFailed(String),

    #[error("failed to remove container: {0}")]
    RemoveFailed(String),

    #[error("failed to get container status: {0}")]
    StatusFailed(String),
}

/// Status of a container as reported by the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerStatus {
    Running,
    Stopped,
    NotFound,
    Unknown(String),
}

/// Trait for container runtime operations (e.g., podman).
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Start a container with the given name from the specified image.
    async fn run(&self, name: &str, image_ref: &str) -> Result<(), ContainerError>;

    /// Stop a running container by name.
    async fn stop(&self, name: &str) -> Result<(), ContainerError>;

    /// Force-remove a container by name.
    async fn remove(&self, name: &str) -> Result<(), ContainerError>;

    /// Query the status of a container by name.
    async fn status(&self, name: &str) -> Result<ContainerStatus, ContainerError>;
}

/// Container runtime implementation using podman CLI.
///
/// All operations shell out to `podman` via `tokio::process::Command`.
pub struct PodmanRuntime;

impl PodmanRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PodmanRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContainerRuntime for PodmanRuntime {
    async fn run(&self, name: &str, image_ref: &str) -> Result<(), ContainerError> {
        debug!(name, image_ref, "running container via podman");

        let output = tokio::process::Command::new("podman")
            .args(["run", "-d", "--name", name, image_ref])
            .output()
            .await
            .map_err(|e| {
                ContainerError::RunFailed(format!("failed to execute podman run: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(ContainerError::RunFailed(format!(
                "exit code {exit_code}: {}",
                stderr.trim()
            )));
        }

        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<(), ContainerError> {
        debug!(name, "stopping container via podman");

        let output = tokio::process::Command::new("podman")
            .args(["stop", name])
            .output()
            .await
            .map_err(|e| {
                ContainerError::StopFailed(format!("failed to execute podman stop: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::StopFailed(stderr.trim().to_string()));
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<(), ContainerError> {
        debug!(name, "force-removing container via podman");

        let output = tokio::process::Command::new("podman")
            .args(["rm", "-f", name])
            .output()
            .await
            .map_err(|e| {
                ContainerError::RemoveFailed(format!("failed to execute podman rm: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::RemoveFailed(stderr.trim().to_string()));
        }

        Ok(())
    }

    async fn status(&self, name: &str) -> Result<ContainerStatus, ContainerError> {
        debug!(name, "querying container status via podman");

        let output = tokio::process::Command::new("podman")
            .args(["inspect", name, "--format", "{{.State.Status}}"])
            .output()
            .await
            .map_err(|e| {
                ContainerError::StatusFailed(format!("failed to execute podman inspect: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr_str = stderr.trim();
            // podman inspect fails with "no such container" when container doesn't exist
            if stderr_str.contains("no such") || stderr_str.contains("not found") {
                return Ok(ContainerStatus::NotFound);
            }
            return Err(ContainerError::StatusFailed(stderr_str.to_string()));
        }

        let status_str = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_lowercase();

        let container_status = match status_str.as_str() {
            "running" => ContainerStatus::Running,
            "exited" | "stopped" | "dead" => ContainerStatus::Stopped,
            other => {
                warn!(name, status = other, "unexpected container status");
                ContainerStatus::Unknown(other.to_string())
            }
        };

        Ok(container_status)
    }
}

#[cfg(test)]
#[path = "container_test.rs"]
mod container_test;
