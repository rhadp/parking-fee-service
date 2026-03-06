use std::fmt;

/// Errors from container runtime operations.
#[derive(Debug, Clone)]
pub enum ContainerError {
    /// Container failed to start.
    StartFailed(String),
    /// Container failed to stop.
    StopFailed(String),
    /// Container removal failed.
    RemoveFailed(String),
    /// Status query failed.
    StatusFailed(String),
    /// Container not found.
    NotFound(String),
}

impl fmt::Display for ContainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerError::StartFailed(msg) => write!(f, "container failed to start: {msg}"),
            ContainerError::StopFailed(msg) => write!(f, "container failed to stop: {msg}"),
            ContainerError::RemoveFailed(msg) => write!(f, "container removal failed: {msg}"),
            ContainerError::StatusFailed(msg) => write!(f, "container status failed: {msg}"),
            ContainerError::NotFound(msg) => write!(f, "container not found: {msg}"),
        }
    }
}

impl std::error::Error for ContainerError {}

/// Container status as reported by the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerStatus {
    Running,
    Stopped,
    NotFound,
    Unknown(String),
}

/// Trait for container runtime operations (podman).
#[mockall::automock]
#[async_trait::async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Start a container from an image.
    async fn run(&self, name: &str, image_ref: &str) -> Result<(), ContainerError>;

    /// Stop a running container.
    async fn stop(&self, name: &str) -> Result<(), ContainerError>;

    /// Force remove a container.
    async fn remove(&self, name: &str) -> Result<(), ContainerError>;

    /// Query the status of a container.
    async fn status(&self, name: &str) -> Result<ContainerStatus, ContainerError>;
}

/// Container runtime backed by the podman CLI.
///
/// Executes podman commands via `tokio::process::Command` for async compatibility.
/// All errors from podman (non-zero exit codes, stderr output) are captured and
/// mapped to appropriate `ContainerError` variants.
pub struct PodmanRuntime;

impl PodmanRuntime {
    /// Create a new PodmanRuntime instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PodmanRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse podman status output string into a `ContainerStatus`.
fn parse_container_status(raw: &str) -> ContainerStatus {
    match raw.trim().to_lowercase().as_str() {
        "running" => ContainerStatus::Running,
        "exited" | "stopped" | "created" | "paused" | "dead" => ContainerStatus::Stopped,
        "" => ContainerStatus::NotFound,
        other => ContainerStatus::Unknown(other.to_string()),
    }
}

#[async_trait::async_trait]
impl ContainerRuntime for PodmanRuntime {
    /// Start a container using `podman run -d --name <name> <image_ref>`.
    async fn run(&self, name: &str, image_ref: &str) -> Result<(), ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args(["run", "-d", "--name", name, image_ref])
            .output()
            .await
            .map_err(|e| ContainerError::StartFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::StartFailed(format!(
                "exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )));
        }

        Ok(())
    }

    /// Stop a running container using `podman stop <name>`.
    async fn stop(&self, name: &str) -> Result<(), ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args(["stop", name])
            .output()
            .await
            .map_err(|e| ContainerError::StopFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr_str = stderr.trim();
            // If container is not found, return NotFound
            if stderr_str.contains("no such container")
                || stderr_str.contains("not found")
                || stderr_str.contains("no container with name or ID")
            {
                return Err(ContainerError::NotFound(name.to_string()));
            }
            return Err(ContainerError::StopFailed(format!(
                "exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr_str
            )));
        }

        Ok(())
    }

    /// Force remove a container using `podman rm -f <name>`.
    async fn remove(&self, name: &str) -> Result<(), ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args(["rm", "-f", name])
            .output()
            .await
            .map_err(|e| ContainerError::RemoveFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr_str = stderr.trim();
            if stderr_str.contains("no such container")
                || stderr_str.contains("not found")
                || stderr_str.contains("no container with name or ID")
            {
                return Err(ContainerError::NotFound(name.to_string()));
            }
            return Err(ContainerError::RemoveFailed(format!(
                "exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr_str
            )));
        }

        Ok(())
    }

    /// Query the status of a container using `podman inspect <name> --format '{{.State.Status}}'`.
    async fn status(&self, name: &str) -> Result<ContainerStatus, ContainerError> {
        let output = tokio::process::Command::new("podman")
            .args(["inspect", name, "--format", "{{.State.Status}}"])
            .output()
            .await
            .map_err(|e| ContainerError::StatusFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr_str = stderr.trim();
            if stderr_str.contains("no such container")
                || stderr_str.contains("not found")
                || stderr_str.contains("no container with name or ID")
            {
                return Ok(ContainerStatus::NotFound);
            }
            return Err(ContainerError::StatusFailed(format!(
                "exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr_str
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_container_status(&stdout))
    }
}

#[cfg(test)]
#[path = "container_test.rs"]
mod tests;
