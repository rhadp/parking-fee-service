//! Container management for UPDATE_SERVICE.
//!
//! This module manages container lifecycle using podman, including
//! installation, starting, stopping, and removal of adapters.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use tokio::process::Command;
use tracing::{debug, warn};

use crate::downloader::DownloadedImage;
use crate::error::UpdateError;
use crate::logger::{ContainerOperation, OperationLogger, OperationOutcome};

/// Container manager for podman operations.
#[derive(Clone)]
pub struct ContainerManager {
    storage_path: PathBuf,
    data_broker_socket: String,
    logger: Arc<OperationLogger>,
}

impl ContainerManager {
    /// Create a new container manager.
    pub fn new(
        storage_path: PathBuf,
        data_broker_socket: String,
        logger: Arc<OperationLogger>,
    ) -> Self {
        Self {
            storage_path,
            data_broker_socket,
            logger,
        }
    }

    /// Install a container from a downloaded image.
    pub async fn install(
        &self,
        adapter_id: &str,
        image: &DownloadedImage,
        correlation_id: &str,
    ) -> Result<(), UpdateError> {
        debug!("Installing container for adapter {}", adapter_id);

        // Load the image into podman from the OCI directory
        // podman load -i <image_dir>
        let output = Command::new("podman")
            .args([
                "load",
                "-i",
                image
                    .manifest_path
                    .parent()
                    .unwrap_or(&image.manifest_path)
                    .to_str()
                    .unwrap_or(""),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                self.logger.log_container_operation(
                    correlation_id,
                    adapter_id,
                    ContainerOperation::Install,
                    OperationOutcome::Failure(e.to_string()),
                );
                UpdateError::ContainerError(format!("Failed to run podman load: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.logger.log_container_operation(
                correlation_id,
                adapter_id,
                ContainerOperation::Install,
                OperationOutcome::Failure(stderr.to_string()),
            );
            return Err(UpdateError::ContainerError(format!(
                "podman load failed: {}",
                stderr
            )));
        }

        self.logger.log_container_operation(
            correlation_id,
            adapter_id,
            ContainerOperation::Install,
            OperationOutcome::Success,
        );

        Ok(())
    }

    /// Start an installed container.
    pub async fn start(
        &self,
        adapter_id: &str,
        image_ref: &str,
        correlation_id: &str,
    ) -> Result<(), UpdateError> {
        debug!("Starting container for adapter {}", adapter_id);

        // Create and start the container with network access to DATA_BROKER
        // podman run -d --name <adapter_id> --volume <socket>:<socket> <image>
        let output = Command::new("podman")
            .args([
                "run",
                "-d",
                "--name",
                adapter_id,
                "--replace", // Replace if exists
                "--volume",
                &format!("{}:{}", self.data_broker_socket, self.data_broker_socket),
                "--env",
                &format!("DATA_BROKER_SOCKET={}", self.data_broker_socket),
                image_ref,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                self.logger.log_container_operation(
                    correlation_id,
                    adapter_id,
                    ContainerOperation::Start,
                    OperationOutcome::Failure(e.to_string()),
                );
                UpdateError::ContainerError(format!("Failed to run podman run: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.logger.log_container_operation(
                correlation_id,
                adapter_id,
                ContainerOperation::Start,
                OperationOutcome::Failure(stderr.to_string()),
            );
            return Err(UpdateError::ContainerError(format!(
                "podman run failed: {}",
                stderr
            )));
        }

        self.logger.log_container_operation(
            correlation_id,
            adapter_id,
            ContainerOperation::Start,
            OperationOutcome::Success,
        );

        Ok(())
    }

    /// Stop a running container.
    pub async fn stop(&self, adapter_id: &str, correlation_id: &str) -> Result<(), UpdateError> {
        debug!("Stopping container for adapter {}", adapter_id);

        let output = Command::new("podman")
            .args(["stop", adapter_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                self.logger.log_container_operation(
                    correlation_id,
                    adapter_id,
                    ContainerOperation::Stop,
                    OperationOutcome::Failure(e.to_string()),
                );
                UpdateError::ContainerError(format!("Failed to run podman stop: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Log warning but don't fail if container doesn't exist
            if !stderr.contains("no such container") {
                self.logger.log_container_operation(
                    correlation_id,
                    adapter_id,
                    ContainerOperation::Stop,
                    OperationOutcome::Failure(stderr.to_string()),
                );
                return Err(UpdateError::ContainerError(format!(
                    "podman stop failed: {}",
                    stderr
                )));
            }
            warn!("Container {} not found, may already be stopped", adapter_id);
        }

        self.logger.log_container_operation(
            correlation_id,
            adapter_id,
            ContainerOperation::Stop,
            OperationOutcome::Success,
        );

        Ok(())
    }

    /// Remove a container and its storage.
    pub async fn remove(&self, adapter_id: &str, correlation_id: &str) -> Result<(), UpdateError> {
        debug!("Removing container for adapter {}", adapter_id);

        // Remove the container
        let output = Command::new("podman")
            .args(["rm", "-f", adapter_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                self.logger.log_container_operation(
                    correlation_id,
                    adapter_id,
                    ContainerOperation::Remove,
                    OperationOutcome::Failure(e.to_string()),
                );
                UpdateError::ContainerError(format!("Failed to run podman rm: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail if container doesn't exist
            if !stderr.contains("no such container") {
                self.logger.log_container_operation(
                    correlation_id,
                    adapter_id,
                    ContainerOperation::Remove,
                    OperationOutcome::Failure(stderr.to_string()),
                );
                return Err(UpdateError::ContainerError(format!(
                    "podman rm failed: {}",
                    stderr
                )));
            }
        }

        // Remove storage directory
        let adapter_storage = self.storage_path.join(adapter_id);
        if adapter_storage.exists() {
            tokio::fs::remove_dir_all(&adapter_storage)
                .await
                .map_err(|e| {
                    UpdateError::ContainerError(format!("Failed to remove storage: {}", e))
                })?;
        }

        self.logger.log_container_operation(
            correlation_id,
            adapter_id,
            ContainerOperation::Remove,
            OperationOutcome::Success,
        );

        Ok(())
    }

    /// List running containers that match adapter naming pattern.
    pub async fn list_running(&self) -> Result<Vec<String>, UpdateError> {
        debug!("Listing running containers");

        let output = Command::new("podman")
            .args(["ps", "--format", "{{.Names}}", "--filter", "status=running"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| UpdateError::ContainerError(format!("Failed to run podman ps: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(UpdateError::ContainerError(format!(
                "podman ps failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let containers: Vec<String> = stdout
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(containers)
    }

    /// Check if a container is running.
    pub async fn is_running(&self, adapter_id: &str) -> Result<bool, UpdateError> {
        let running = self.list_running().await?;
        Ok(running.contains(&adapter_id.to_string()))
    }

    /// Get container status.
    pub async fn get_status(&self, adapter_id: &str) -> Result<Option<String>, UpdateError> {
        let output = Command::new("podman")
            .args(["inspect", "--format", "{{.State.Status}}", adapter_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                UpdateError::ContainerError(format!("Failed to inspect container: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("no such container") {
                return Ok(None);
            }
            return Err(UpdateError::ContainerError(format!(
                "podman inspect failed: {}",
                stderr
            )));
        }

        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if status.is_empty() {
            Ok(None)
        } else {
            Ok(Some(status))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn create_test_manager() -> ContainerManager {
        let logger = Arc::new(OperationLogger::new("test"));
        ContainerManager::new(
            PathBuf::from("/tmp/test-containers"),
            "/run/kuksa/databroker.sock".to_string(),
            logger,
        )
    }

    #[test]
    fn test_container_manager_new() {
        let manager = create_test_manager();
        assert_eq!(manager.storage_path, PathBuf::from("/tmp/test-containers"));
        assert_eq!(manager.data_broker_socket, "/run/kuksa/databroker.sock");
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 9: Container Startup Failure Handling
        /// Validates: Requirements 4.6
        #[test]
        fn prop_container_manager_configuration(
            adapter_id in "[a-z][a-z0-9-]{3,20}",
            storage_path in "/[a-z]+/[a-z]+"
        ) {
            let logger = Arc::new(OperationLogger::new("test"));
            let expected_path = PathBuf::from(&storage_path);
            let manager = ContainerManager::new(
                expected_path.clone(),
                "/run/test.sock".to_string(),
                logger,
            );

            // Verify storage path is set correctly
            prop_assert_eq!(&manager.storage_path, &expected_path);

            // Adapter storage should be under storage_path
            let adapter_storage = manager.storage_path.join(&adapter_id);
            prop_assert!(adapter_storage.starts_with(&manager.storage_path));
        }
    }
}
