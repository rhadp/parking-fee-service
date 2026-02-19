//! Podman CLI wrapper for container lifecycle management.
//!
//! This module provides a trait-based abstraction over podman CLI commands
//! for creating, starting, stopping, removing, and inspecting containers.
//! The trait design enables unit testing with mock command executors.
//!
//! # Requirements
//!
//! - 04-REQ-3.1: Create and start a container using podman.
//! - 04-REQ-3.3: Stop and remove a container using podman.
//! - 04-REQ-3.6: Inspect running containers for state reconciliation.

use std::collections::HashMap;

use thiserror::Error;
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from podman operations.
#[derive(Debug, Error)]
pub enum PodmanError {
    /// A podman command failed.
    #[error("podman {command} failed (exit={exit_code}): {stderr}")]
    CommandFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },

    /// Failed to execute the podman binary.
    #[error("failed to execute podman: {0}")]
    ExecError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, PodmanError>;

// ---------------------------------------------------------------------------
// ContainerInfo — output from `podman ps`
// ---------------------------------------------------------------------------

/// Summary information about a running container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerInfo {
    /// Container name.
    pub name: String,
    /// Whether the container is currently running.
    pub running: bool,
}

// ---------------------------------------------------------------------------
// ContainerRuntime trait — abstraction for testability
// ---------------------------------------------------------------------------

/// Abstraction over container runtime operations.
///
/// The default implementation uses the `podman` CLI. Unit tests inject
/// a mock implementation.
///
/// All methods return `Send` futures so they can be used inside
/// `tokio::spawn` and `tonic::async_trait` service methods.
pub trait ContainerRuntime: Send + Sync {
    /// Create and start a container.
    ///
    /// # Arguments
    ///
    /// * `name` — Container name (e.g. `poa-adapter-001`).
    /// * `image` — Image reference (e.g. `localhost/parking-operator-adaptor:latest`).
    /// * `env_vars` — Environment variables to pass to the container.
    /// * `network` — Podman network to attach (e.g. `host`).
    fn create_and_start(
        &self,
        name: &str,
        image: &str,
        env_vars: &HashMap<String, String>,
        network: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Stop and remove a container.
    fn stop_and_remove(&self, name: &str) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Check if a container is currently running.
    fn is_running(&self, name: &str) -> impl std::future::Future<Output = Result<bool>> + Send;

    /// List containers whose names start with `prefix`.
    fn list(
        &self,
        prefix: &str,
    ) -> impl std::future::Future<Output = Result<Vec<ContainerInfo>>> + Send;
}

// ---------------------------------------------------------------------------
// PodmanRunner — real implementation using podman CLI
// ---------------------------------------------------------------------------

/// Real podman CLI implementation of [`ContainerRuntime`].
#[derive(Debug, Clone, Default)]
pub struct PodmanRunner;

impl PodmanRunner {
    /// Execute a podman command and return (stdout, stderr).
    async fn exec(args: &[&str]) -> Result<String> {
        debug!(args = ?args, "executing podman command");

        let output = tokio::process::Command::new("podman")
            .args(args)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let command = args.join(" ");
            error!(command = %command, exit_code, stderr = %stderr, "podman command failed");
            return Err(PodmanError::CommandFailed {
                command,
                exit_code,
                stderr: stderr.trim().to_string(),
            });
        }

        Ok(stdout)
    }
}

impl ContainerRuntime for PodmanRunner {
    async fn create_and_start(
        &self,
        name: &str,
        image: &str,
        env_vars: &HashMap<String, String>,
        network: &str,
    ) -> Result<()> {
        info!(name, image, network, "creating container");

        // Build the create command
        let mut args = vec!["create", "--name", name, "--network", network];

        // Collect env var strings so they live long enough
        let env_strings: Vec<String> = env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        for env_str in &env_strings {
            args.push("-e");
            args.push(env_str);
        }

        args.push(image);

        Self::exec(&args).await?;

        info!(name, "starting container");
        Self::exec(&["start", name]).await?;

        info!(name, "container created and started");
        Ok(())
    }

    async fn stop_and_remove(&self, name: &str) -> Result<()> {
        info!(name, "stopping container");

        // Stop (ignore error if already stopped)
        match Self::exec(&["stop", name]).await {
            Ok(_) => {}
            Err(PodmanError::CommandFailed { ref stderr, .. })
                if stderr.contains("no such container")
                    || stderr.contains("no container with") =>
            {
                warn!(name, "container not found during stop, continuing to remove");
            }
            Err(e) => return Err(e),
        }

        info!(name, "removing container");
        match Self::exec(&["rm", name]).await {
            Ok(_) => {}
            Err(PodmanError::CommandFailed { ref stderr, .. })
                if stderr.contains("no such container")
                    || stderr.contains("no container with") =>
            {
                warn!(name, "container not found during remove");
            }
            Err(e) => return Err(e),
        }

        info!(name, "container stopped and removed");
        Ok(())
    }

    async fn is_running(&self, name: &str) -> Result<bool> {
        match Self::exec(&[
            "inspect",
            "--format",
            "{{.State.Running}}",
            name,
        ])
        .await
        {
            Ok(output) => Ok(output.trim() == "true"),
            Err(PodmanError::CommandFailed { ref stderr, .. })
                if stderr.contains("no such container")
                    || stderr.contains("no container with") =>
            {
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    async fn list(&self, prefix: &str) -> Result<Vec<ContainerInfo>> {
        let filter = format!("name=^{}", prefix);
        let output = Self::exec(&[
            "ps",
            "--all",
            "--filter",
            &filter,
            "--format",
            "{{.Names}} {{.State}}",
        ])
        .await?;

        let mut containers = Vec::new();
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() >= 2 {
                containers.push(ContainerInfo {
                    name: parts[0].to_string(),
                    running: parts[1].to_lowercase().contains("running"),
                });
            } else if !parts.is_empty() {
                containers.push(ContainerInfo {
                    name: parts[0].to_string(),
                    running: false,
                });
            }
        }

        Ok(containers)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Record of a podman operation for verification in tests.
    #[derive(Debug, Clone, PartialEq)]
    pub enum PodmanCall {
        CreateAndStart {
            name: String,
            image: String,
            env_vars: HashMap<String, String>,
            network: String,
        },
        StopAndRemove {
            name: String,
        },
        IsRunning {
            name: String,
        },
        List {
            prefix: String,
        },
    }

    /// Mock container runtime that records calls and returns configurable results.
    #[derive(Debug, Clone)]
    pub struct MockContainerRuntime {
        pub calls: Arc<Mutex<Vec<PodmanCall>>>,
        /// If true, create_and_start will fail.
        pub fail_create: bool,
        /// If true, stop_and_remove will fail.
        pub fail_stop: bool,
        /// Containers that are "running" for is_running checks.
        pub running_containers: Arc<Mutex<Vec<String>>>,
        /// Pre-configured list results.
        pub list_results: Vec<ContainerInfo>,
    }

    impl Default for MockContainerRuntime {
        fn default() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                fail_create: false,
                fail_stop: false,
                running_containers: Arc::new(Mutex::new(Vec::new())),
                list_results: Vec::new(),
            }
        }
    }

    impl MockContainerRuntime {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_fail_create(mut self) -> Self {
            self.fail_create = true;
            self
        }

        pub fn with_running(self, name: &str) -> Self {
            self.running_containers
                .lock()
                .unwrap()
                .push(name.to_string());
            self
        }

        pub fn get_calls(&self) -> Vec<PodmanCall> {
            self.calls.lock().unwrap().clone()
        }

        pub fn add_running(&self, name: &str) {
            self.running_containers
                .lock()
                .unwrap()
                .push(name.to_string());
        }

        pub fn remove_running(&self, name: &str) {
            self.running_containers
                .lock()
                .unwrap()
                .retain(|n| n != name);
        }
    }

    impl ContainerRuntime for MockContainerRuntime {
        async fn create_and_start(
            &self,
            name: &str,
            image: &str,
            env_vars: &HashMap<String, String>,
            network: &str,
        ) -> Result<()> {
            self.calls.lock().unwrap().push(PodmanCall::CreateAndStart {
                name: name.to_string(),
                image: image.to_string(),
                env_vars: env_vars.clone(),
                network: network.to_string(),
            });

            if self.fail_create {
                return Err(PodmanError::CommandFailed {
                    command: "create".to_string(),
                    exit_code: 125,
                    stderr: "Error: localhost/nonexistent:latest: image not known".to_string(),
                });
            }

            // Track as running
            self.running_containers
                .lock()
                .unwrap()
                .push(name.to_string());

            Ok(())
        }

        async fn stop_and_remove(&self, name: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(PodmanCall::StopAndRemove {
                    name: name.to_string(),
                });

            if self.fail_stop {
                return Err(PodmanError::CommandFailed {
                    command: "stop".to_string(),
                    exit_code: 125,
                    stderr: "error stopping container".to_string(),
                });
            }

            // Track as not running
            self.running_containers
                .lock()
                .unwrap()
                .retain(|n| n != name);

            Ok(())
        }

        async fn is_running(&self, name: &str) -> Result<bool> {
            self.calls.lock().unwrap().push(PodmanCall::IsRunning {
                name: name.to_string(),
            });
            let running = self
                .running_containers
                .lock()
                .unwrap()
                .contains(&name.to_string());
            Ok(running)
        }

        async fn list(&self, prefix: &str) -> Result<Vec<ContainerInfo>> {
            self.calls.lock().unwrap().push(PodmanCall::List {
                prefix: prefix.to_string(),
            });
            Ok(self.list_results.clone())
        }
    }

    // ---- Unit tests for the mock ----

    #[tokio::test]
    async fn mock_create_and_start_records_call() {
        let mock = MockContainerRuntime::new();
        let env = HashMap::from([("KEY".to_string(), "VALUE".to_string())]);

        mock.create_and_start("test-container", "my-image:latest", &env, "host")
            .await
            .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert!(matches!(
            &calls[0],
            PodmanCall::CreateAndStart { name, image, .. }
            if name == "test-container" && image == "my-image:latest"
        ));

        // Should be tracked as running
        assert!(mock.is_running("test-container").await.unwrap());
    }

    #[tokio::test]
    async fn mock_create_failure() {
        let mock = MockContainerRuntime::new().with_fail_create();
        let env = HashMap::new();

        let result = mock
            .create_and_start("test", "bad-image:latest", &env, "host")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_stop_and_remove_records_call() {
        let mock = MockContainerRuntime::new().with_running("test-container");

        assert!(mock.is_running("test-container").await.unwrap());

        mock.stop_and_remove("test-container").await.unwrap();

        assert!(!mock.is_running("test-container").await.unwrap());
    }

    #[tokio::test]
    async fn mock_is_running_returns_false_for_unknown() {
        let mock = MockContainerRuntime::new();
        assert!(!mock.is_running("nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn mock_list_returns_configured_results() {
        let mock = MockContainerRuntime {
            list_results: vec![
                ContainerInfo {
                    name: "poa-a1".to_string(),
                    running: true,
                },
                ContainerInfo {
                    name: "poa-a2".to_string(),
                    running: false,
                },
            ],
            ..Default::default()
        };

        let results = mock.list("poa-").await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].running);
        assert!(!results[1].running);
    }

    #[tokio::test]
    async fn mock_env_vars_passed_correctly() {
        let mock = MockContainerRuntime::new();
        let env = HashMap::from([
            ("DATABROKER_ADDR".to_string(), "localhost:55555".to_string()),
            (
                "PARKING_OPERATOR_URL".to_string(),
                "http://op:8082".to_string(),
            ),
            ("ZONE_ID".to_string(), "zone-1".to_string()),
        ]);

        mock.create_and_start("test", "img:latest", &env, "host")
            .await
            .unwrap();

        let calls = mock.get_calls();
        if let PodmanCall::CreateAndStart {
            env_vars, network, ..
        } = &calls[0]
        {
            assert_eq!(env_vars.len(), 3);
            assert_eq!(env_vars.get("ZONE_ID").unwrap(), "zone-1");
            assert_eq!(network, "host");
        } else {
            panic!("expected CreateAndStart call");
        }
    }

    #[test]
    fn podman_error_display() {
        let err = PodmanError::CommandFailed {
            command: "create".to_string(),
            exit_code: 125,
            stderr: "image not found".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("create"));
        assert!(msg.contains("125"));
        assert!(msg.contains("image not found"));
    }
}
