use async_trait::async_trait;
use std::fmt;

/// Error returned by podman operations.
#[derive(Clone, Debug)]
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

impl fmt::Display for PodmanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// Real podman executor that shells out to podman CLI via tokio::process::Command.
pub struct RealPodmanExecutor;

impl RealPodmanExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Run a podman command and return stdout on success or PodmanError on failure.
    async fn run_command(
        &self,
        args: &[&str],
    ) -> Result<String, PodmanError> {
        let output = tokio::process::Command::new("podman")
            .args(args)
            .output()
            .await
            .map_err(|e| PodmanError::new(&format!("failed to execute podman: {e}")))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(PodmanError::new(&stderr))
        }
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
        self.run_command(&["pull", image_ref]).await?;
        Ok(())
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        self.run_command(&["image", "inspect", "--format", "{{.Digest}}", image_ref])
            .await
    }

    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
        self.run_command(&[
            "run", "-d", "--name", adapter_id, "--network=host", image_ref,
        ])
        .await?;
        Ok(())
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        self.run_command(&["stop", adapter_id]).await?;
        Ok(())
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        self.run_command(&["rm", adapter_id]).await?;
        Ok(())
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        self.run_command(&["rmi", image_ref]).await?;
        Ok(())
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        let output = self.run_command(&["wait", adapter_id]).await?;
        output
            .parse::<i32>()
            .map_err(|e| PodmanError::new(&format!("failed to parse exit code: {e}")))
    }
}
