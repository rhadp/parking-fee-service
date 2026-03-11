use async_trait::async_trait;
use thiserror::Error;

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
