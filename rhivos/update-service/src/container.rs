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
