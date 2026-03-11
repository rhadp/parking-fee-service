use std::sync::Arc;

use crate::container::MockContainerRuntime;
use crate::oci::{MockOciPuller, OciError, PullResult};
use crate::state::AdapterState;

use super::*;

/// Helper: create mock OCI puller that returns a successful pull with the given digest.
fn mock_oci_success(digest: &str) -> MockOciPuller {
    let digest = digest.to_string();
    let mut mock = MockOciPuller::new();
    mock.expect_pull_image()
        .returning(move |_| Ok(PullResult { digest: digest.clone() }));
    mock.expect_remove_image()
        .returning(|_| Ok(()));
    mock
}

/// Helper: create mock OCI puller that fails with a connection error.
fn mock_oci_unavailable() -> MockOciPuller {
    let mut mock = MockOciPuller::new();
    mock.expect_pull_image()
        .returning(|_| Err(OciError::PullFailed("connection refused".into())));
    mock
}

/// Helper: create mock container runtime that succeeds on all operations.
fn mock_container_success() -> MockContainerRuntime {
    let mut mock = MockContainerRuntime::new();
    mock.expect_run().returning(|_, _| Ok(()));
    mock.expect_stop().returning(|_| Ok(()));
    mock.expect_remove().returning(|_| Ok(()));
    mock.expect_status()
        .returning(|_| Ok(crate::container::ContainerStatus::Running));
    mock
}

/// Helper: create mock container runtime that fails on run.
fn mock_container_run_fails() -> MockContainerRuntime {
    let mut mock = MockContainerRuntime::new();
    mock.expect_run()
        .returning(|_, _| Err(crate::container::ContainerError::RunFailed("exit code 125".into())));
    mock.expect_stop().returning(|_| Ok(()));
    mock.expect_remove().returning(|_| Ok(()));
    mock
}

/// Compute the expected SHA-256 checksum of a digest string for test fixtures.
fn compute_test_checksum(digest: &str) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(digest.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

// TS-07-1: Install adapter happy path
#[tokio::test]
async fn test_install_adapter_happy_path() {
    let digest = "sha256:abc123def456";
    let expected_checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    let result = manager
        .install_adapter("test-registry/test-image:v1.0", &expected_checksum)
        .await;

    assert!(result.is_ok(), "install_adapter should succeed: {:?}", result.err());
    let install = result.unwrap();
    assert!(!install.job_id.is_empty(), "job_id should not be empty");
    assert!(!install.adapter_id.is_empty(), "adapter_id should not be empty");
    assert_eq!(install.state, AdapterState::Downloading, "initial state should be DOWNLOADING");

    // After async processing, the adapter should eventually be RUNNING.
    // Give a small delay for background processing.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let status = manager.get_adapter_status(&install.adapter_id).await;
    assert!(status.is_ok(), "get_adapter_status should succeed");
    assert_eq!(status.unwrap().state, AdapterState::Running, "adapter should reach RUNNING");
}

// TS-07-4: Single adapter enforcement
#[tokio::test]
async fn test_single_adapter_enforcement() {
    let digest_a = "sha256:aaa111";
    let checksum_a = compute_test_checksum(digest_a);
    let digest_b = "sha256:bbb222";
    let checksum_b = compute_test_checksum(digest_b);

    let mut oci = MockOciPuller::new();
    let digest_a_clone = digest_a.to_string();
    let digest_b_clone = digest_b.to_string();
    oci.expect_pull_image()
        .returning(move |image_ref: &str| {
            if image_ref.contains("image-a") {
                Ok(PullResult { digest: digest_a_clone.clone() })
            } else {
                Ok(PullResult { digest: digest_b_clone.clone() })
            }
        });
    oci.expect_remove_image().returning(|_| Ok(()));

    let container = mock_container_success();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    // Install adapter A
    let result_a = manager.install_adapter("test-registry/image-a:v1.0", &checksum_a).await;
    assert!(result_a.is_ok(), "install adapter A should succeed");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Install adapter B (should stop A first)
    let result_b = manager.install_adapter("test-registry/image-b:v1.0", &checksum_b).await;
    assert!(result_b.is_ok(), "install adapter B should succeed");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let adapters = manager.list_adapters().await;
    let running_count = adapters.iter().filter(|a| a.state == AdapterState::Running).count();
    assert_eq!(running_count, 1, "exactly one adapter should be RUNNING");

    // Adapter A should be stopped
    let adapter_a = adapters.iter().find(|a| a.image_ref.contains("image-a"));
    assert!(adapter_a.is_some(), "adapter A should still exist");
    assert_eq!(adapter_a.unwrap().state, AdapterState::Stopped, "adapter A should be STOPPED");

    // Adapter B should be running
    let adapter_b = adapters.iter().find(|a| a.image_ref.contains("image-b"));
    assert!(adapter_b.is_some(), "adapter B should exist");
    assert_eq!(adapter_b.unwrap().state, AdapterState::Running, "adapter B should be RUNNING");
}

// TS-07-5: List adapters
#[tokio::test]
async fn test_list_adapters() {
    let digest_a = "sha256:aaa111";
    let checksum_a = compute_test_checksum(digest_a);
    let digest_b = "sha256:bbb222";
    let checksum_b = compute_test_checksum(digest_b);

    let mut oci = MockOciPuller::new();
    let da = digest_a.to_string();
    let db = digest_b.to_string();
    oci.expect_pull_image()
        .returning(move |image_ref: &str| {
            if image_ref.contains("image-a") {
                Ok(PullResult { digest: da.clone() })
            } else {
                Ok(PullResult { digest: db.clone() })
            }
        });
    oci.expect_remove_image().returning(|_| Ok(()));

    let container = mock_container_success();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    // Install two adapters
    manager.install_adapter("test-registry/image-a:v1.0", &checksum_a).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    manager.install_adapter("test-registry/image-b:v1.0", &checksum_b).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let adapters = manager.list_adapters().await;
    assert_eq!(adapters.len(), 2, "should have 2 adapters");

    // One RUNNING, one STOPPED
    let running = adapters.iter().filter(|a| a.state == AdapterState::Running).count();
    let stopped = adapters.iter().filter(|a| a.state == AdapterState::Stopped).count();
    assert_eq!(running, 1, "one adapter should be RUNNING");
    assert_eq!(stopped, 1, "one adapter should be STOPPED");

    // Each adapter should have valid fields
    for adapter in &adapters {
        assert!(!adapter.adapter_id.is_empty(), "adapter_id should not be empty");
        assert!(!adapter.image_ref.is_empty(), "image_ref should not be empty");
    }
}

// TS-07-6: Get adapter status
#[tokio::test]
async fn test_get_adapter_status() {
    let digest = "sha256:abc123";
    let checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    let install = manager.install_adapter("test-registry/image:v1.0", &checksum).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let status = manager.get_adapter_status(&install.adapter_id).await;
    assert!(status.is_ok(), "get_adapter_status should succeed");

    let record = status.unwrap();
    assert_eq!(record.adapter_id, install.adapter_id);
    assert_eq!(record.image_ref, "test-registry/image:v1.0");
    assert_eq!(record.state, AdapterState::Running);
}

// TS-07-7: Remove adapter
#[tokio::test]
async fn test_remove_adapter() {
    let digest = "sha256:abc123";
    let checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    let install = manager.install_adapter("test-registry/image:v1.0", &checksum).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let remove_result = manager.remove_adapter(&install.adapter_id).await;
    assert!(remove_result.is_ok(), "remove_adapter should succeed");

    // After removal, adapter should no longer be listed
    let adapters = manager.list_adapters().await;
    let found = adapters.iter().any(|a| a.adapter_id == install.adapter_id);
    assert!(!found, "removed adapter should no longer be listed");
}

// TS-07-E1: Checksum mismatch
#[tokio::test]
async fn test_checksum_mismatch() {
    let digest = "sha256:abc123";
    let wrong_checksum = "sha256:wrong_checksum_value";

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    let result = manager
        .install_adapter("test-registry/image:v1.0", wrong_checksum)
        .await;

    assert!(result.is_err(), "install with wrong checksum should fail");
    match result.unwrap_err() {
        ManagerError::ChecksumMismatch { expected, actual } => {
            assert_eq!(expected, wrong_checksum);
            assert_ne!(actual, wrong_checksum);
        }
        other => panic!("expected ChecksumMismatch, got: {:?}", other),
    }
}

// TS-07-E2: Registry unreachable
#[tokio::test]
async fn test_registry_unreachable() {
    let oci = mock_oci_unavailable();
    let container = mock_container_success();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    let result = manager
        .install_adapter("test-registry/image:v1.0", "sha256:doesntmatter")
        .await;

    assert!(result.is_err(), "install should fail when registry is unreachable");
    match result.unwrap_err() {
        ManagerError::RegistryUnavailable(msg) => {
            assert!(
                msg.contains("connection refused"),
                "error should mention connection issue: {}",
                msg
            );
        }
        other => panic!("expected RegistryUnavailable, got: {:?}", other),
    }
}

// TS-07-E3: Container start failure
#[tokio::test]
async fn test_container_start_failure() {
    let digest = "sha256:abc123";
    let checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_run_fails();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    let result = manager
        .install_adapter("test-registry/image:v1.0", &checksum)
        .await;

    // The install call returns DOWNLOADING initially, but the adapter should
    // end up in ERROR state after the background task runs.
    // For synchronous error handling, the manager may return an error directly.
    // We accept either: direct error or adapter ending up in ERROR state.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    if let Ok(install) = &result {
        let status = manager.get_adapter_status(&install.adapter_id).await;
        assert!(status.is_ok());
        assert_eq!(
            status.unwrap().state,
            AdapterState::Error,
            "adapter should be in ERROR state after container start failure"
        );
    } else {
        match result.unwrap_err() {
            ManagerError::ContainerStartFailed(msg) => {
                assert!(msg.contains("exit code 125"), "error should mention exit code: {}", msg);
            }
            other => panic!("expected ContainerStartFailed, got: {:?}", other),
        }
    }
}

// TS-07-E4: Get status for unknown adapter
#[tokio::test]
async fn test_get_status_unknown() {
    let oci = MockOciPuller::new();
    let container = MockContainerRuntime::new();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    let result = manager.get_adapter_status("nonexistent-adapter").await;
    assert!(result.is_err(), "get_adapter_status for unknown adapter should fail");
    match result.unwrap_err() {
        ManagerError::NotFound(id) => {
            assert_eq!(id, "nonexistent-adapter");
        }
        other => panic!("expected NotFound, got: {:?}", other),
    }
}

// TS-07-E5: Remove unknown adapter
#[tokio::test]
async fn test_remove_unknown() {
    let oci = MockOciPuller::new();
    let container = MockContainerRuntime::new();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    let result = manager.remove_adapter("nonexistent-adapter").await;
    assert!(result.is_err(), "remove_adapter for unknown adapter should fail");
    match result.unwrap_err() {
        ManagerError::NotFound(id) => {
            assert_eq!(id, "nonexistent-adapter");
        }
        other => panic!("expected NotFound, got: {:?}", other),
    }
}

// TS-07-E6: Install already running adapter (same image)
#[tokio::test]
async fn test_install_already_running() {
    let digest = "sha256:abc123";
    let checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    // Install the first time
    let install = manager.install_adapter("test-registry/image:v1.0", &checksum).await;
    assert!(install.is_ok(), "first install should succeed");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Install again with same image_ref
    let result = manager.install_adapter("test-registry/image:v1.0", &checksum).await;
    assert!(result.is_err(), "second install with same image should fail");
    match result.unwrap_err() {
        ManagerError::AlreadyExists(id) => {
            assert!(!id.is_empty(), "adapter id should be included in AlreadyExists error");
        }
        other => panic!("expected AlreadyExists, got: {:?}", other),
    }
}

// TS-07-P2: Single adapter invariant (property-like test)
#[tokio::test]
async fn test_single_adapter_invariant() {
    // Set up mocks for multiple images
    let mut oci = MockOciPuller::new();
    oci.expect_pull_image()
        .returning(|image_ref: &str| {
            Ok(PullResult {
                digest: format!("sha256:digest-for-{}", image_ref),
            })
        });
    oci.expect_remove_image().returning(|_| Ok(()));

    let container = mock_container_success();
    let manager = AdapterManager::new(Arc::new(oci), Arc::new(container));

    // Perform a sequence of installs, checking invariant after each
    for i in 0..5 {
        let image_ref = format!("test-registry/image-{}:v1.0", i);
        let digest = format!("sha256:digest-for-{}", image_ref);
        let checksum = compute_test_checksum(&digest);

        let _ = manager.install_adapter(&image_ref, &checksum).await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let adapters = manager.list_adapters().await;
        let running_count = adapters.iter().filter(|a| a.state == AdapterState::Running).count();
        assert!(
            running_count <= 1,
            "At most 1 adapter should be RUNNING after install #{}, but found {}",
            i,
            running_count
        );
    }
}
