use std::sync::Arc;

use crate::container::MockContainerRuntime;
use crate::oci::{MockOciPuller, OciError, PullResult};
use crate::state::AdapterState;

use super::{AdapterManager, ManagerError};

/// Helper to create an AdapterManager with mocked dependencies.
fn make_manager(
    oci: MockOciPuller,
    container: MockContainerRuntime,
) -> AdapterManager {
    AdapterManager::new(Arc::new(oci), Arc::new(container))
}

/// Helper to create a mock OCI puller that succeeds for any image.
fn success_oci() -> MockOciPuller {
    let mut mock = MockOciPuller::new();
    mock.expect_pull_image().returning(|_| {
        Ok(PullResult {
            digest: "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
        })
    });
    mock.expect_remove_image().returning(|_| Ok(()));
    mock
}

/// Helper to create a mock container runtime that succeeds.
fn success_container() -> MockContainerRuntime {
    let mut mock = MockContainerRuntime::new();
    mock.expect_run().returning(|_, _| Ok(()));
    mock.expect_stop().returning(|_| Ok(()));
    mock.expect_remove().returning(|_| Ok(()));
    mock.expect_status()
        .returning(|_| Ok(crate::container::ContainerStatus::Running));
    mock
}

/// Valid checksum matching the digest from success_oci().
/// SHA-256 of the digest string "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".
fn valid_checksum() -> String {
    use sha2::{Digest, Sha256};
    let digest_str =
        "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let hash = Sha256::digest(digest_str.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

/// TS-07-1: Install adapter happy path.
#[tokio::test]
async fn test_install_adapter_happy_path() {
    let manager = make_manager(success_oci(), success_container());
    let checksum = valid_checksum();

    let result = manager
        .install_adapter("test-image:v1.0", &checksum)
        .await;

    assert!(result.is_ok(), "install_adapter should succeed");
    let install = result.unwrap();
    assert!(!install.job_id.is_empty(), "job_id should not be empty");
    assert!(
        !install.adapter_id.is_empty(),
        "adapter_id should not be empty"
    );
    assert_eq!(
        install.state,
        AdapterState::Downloading,
        "initial state should be DOWNLOADING"
    );

    // Wait briefly for background processing, then check final state
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let status = manager.get_adapter_status(&install.adapter_id).await;
    assert!(status.is_ok(), "get_adapter_status should succeed");
    assert_eq!(
        status.unwrap().state,
        AdapterState::Running,
        "adapter should reach RUNNING state"
    );
}

/// TS-07-4: Single adapter enforcement.
#[tokio::test]
async fn test_single_adapter_enforcement() {
    let mut oci = MockOciPuller::new();
    oci.expect_pull_image().returning(|_| {
        Ok(PullResult {
            digest: "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
        })
    });
    oci.expect_remove_image().returning(|_| Ok(()));

    let mut container = MockContainerRuntime::new();
    container.expect_run().returning(|_, _| Ok(()));
    container.expect_stop().returning(|_| Ok(()));
    container.expect_remove().returning(|_| Ok(()));
    container
        .expect_status()
        .returning(|_| Ok(crate::container::ContainerStatus::Running));

    let manager = make_manager(oci, container);
    let checksum = valid_checksum();

    // Install adapter A
    let result_a = manager.install_adapter("image-a:v1.0", &checksum).await;
    assert!(result_a.is_ok());
    let adapter_a_id = result_a.unwrap().adapter_id;

    // Wait for A to reach RUNNING
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Install adapter B (should stop A)
    let result_b = manager.install_adapter("image-b:v1.0", &checksum).await;
    assert!(result_b.is_ok());

    // Wait for B to reach RUNNING
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify only one adapter is RUNNING
    let adapters = manager.list_adapters().await;
    let running_count = adapters
        .iter()
        .filter(|a| a.state == AdapterState::Running)
        .count();
    assert_eq!(running_count, 1, "only one adapter should be RUNNING");

    // Verify adapter A is STOPPED
    let status_a = manager.get_adapter_status(&adapter_a_id).await;
    assert!(status_a.is_ok());
    assert_eq!(
        status_a.unwrap().state,
        AdapterState::Stopped,
        "adapter A should be STOPPED"
    );
}

/// TS-07-5: List adapters.
#[tokio::test]
async fn test_list_adapters() {
    let mut oci = MockOciPuller::new();
    oci.expect_pull_image().returning(|_| {
        Ok(PullResult {
            digest: "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
        })
    });
    oci.expect_remove_image().returning(|_| Ok(()));

    let mut container = MockContainerRuntime::new();
    container.expect_run().returning(|_, _| Ok(()));
    container.expect_stop().returning(|_| Ok(()));
    container.expect_remove().returning(|_| Ok(()));
    container
        .expect_status()
        .returning(|_| Ok(crate::container::ContainerStatus::Running));

    let manager = make_manager(oci, container);
    let checksum = valid_checksum();

    // Install two adapters
    manager
        .install_adapter("image-a:v1.0", &checksum)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    manager
        .install_adapter("image-b:v1.0", &checksum)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let adapters = manager.list_adapters().await;
    assert_eq!(adapters.len(), 2, "should list two adapters");

    // One RUNNING, one STOPPED
    let running = adapters
        .iter()
        .filter(|a| a.state == AdapterState::Running)
        .count();
    let stopped = adapters
        .iter()
        .filter(|a| a.state == AdapterState::Stopped)
        .count();
    assert_eq!(running, 1, "one adapter should be RUNNING");
    assert_eq!(stopped, 1, "one adapter should be STOPPED");
}

/// TS-07-6: Get adapter status.
#[tokio::test]
async fn test_get_adapter_status() {
    let manager = make_manager(success_oci(), success_container());
    let checksum = valid_checksum();

    let result = manager
        .install_adapter("test-image:v1.0", &checksum)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let status = manager
        .get_adapter_status(&result.adapter_id)
        .await
        .unwrap();
    assert_eq!(status.adapter_id, result.adapter_id);
    assert_eq!(status.image_ref, "test-image:v1.0");
    assert_eq!(status.state, AdapterState::Running);
}

/// TS-07-7: Remove adapter.
#[tokio::test]
async fn test_remove_adapter() {
    let manager = make_manager(success_oci(), success_container());
    let checksum = valid_checksum();

    let result = manager
        .install_adapter("test-image:v1.0", &checksum)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let remove_result = manager.remove_adapter(&result.adapter_id).await;
    assert!(remove_result.is_ok(), "remove_adapter should succeed");

    // Adapter should no longer be listed (or be in OFFLOADING/removed state)
    let adapters = manager.list_adapters().await;
    let found = adapters
        .iter()
        .any(|a| a.adapter_id == result.adapter_id && a.state == AdapterState::Running);
    assert!(!found, "removed adapter should not be RUNNING");
}

/// TS-07-E1: Checksum mismatch.
#[tokio::test]
async fn test_checksum_mismatch() {
    let manager = make_manager(success_oci(), success_container());

    let result = manager
        .install_adapter("test-image:v1.0", "sha256:badhash")
        .await;

    assert!(result.is_err(), "install should fail with bad checksum");
    match result.unwrap_err() {
        ManagerError::ChecksumMismatch { .. } => {} // expected
        other => panic!("expected ChecksumMismatch, got: {:?}", other),
    }
}

/// TS-07-E2: Registry unreachable.
#[tokio::test]
async fn test_registry_unreachable() {
    let mut oci = MockOciPuller::new();
    oci.expect_pull_image()
        .returning(|_| Err(OciError::RegistryUnavailable("connection refused".into())));
    oci.expect_remove_image().returning(|_| Ok(()));

    let manager = make_manager(oci, success_container());

    let result = manager
        .install_adapter("test-image:v1.0", "sha256:any")
        .await;

    assert!(result.is_err(), "install should fail when registry down");
    match result.unwrap_err() {
        ManagerError::RegistryUnavailable(_) => {} // expected
        other => panic!("expected RegistryUnavailable, got: {:?}", other),
    }
}

/// TS-07-E3: Container start failure.
#[tokio::test]
async fn test_container_start_failure() {
    let mut container = MockContainerRuntime::new();
    container
        .expect_run()
        .returning(|_, _| Err(crate::container::ContainerError::StartFailed("exit code 125".into())));
    container.expect_stop().returning(|_| Ok(()));
    container.expect_remove().returning(|_| Ok(()));
    container
        .expect_status()
        .returning(|_| Ok(crate::container::ContainerStatus::Stopped));

    let manager = make_manager(success_oci(), container);
    let checksum = valid_checksum();

    let result = manager
        .install_adapter("test-image:v1.0", &checksum)
        .await;

    // Depending on implementation, this may fail synchronously or asynchronously.
    // Either the result is an error, or the adapter reaches ERROR state.
    if let Ok(install) = result {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let status = manager
            .get_adapter_status(&install.adapter_id)
            .await
            .unwrap();
        assert_eq!(
            status.state,
            AdapterState::Error,
            "adapter should be in ERROR state after container start failure"
        );
    } else {
        match result.unwrap_err() {
            ManagerError::ContainerFailed(_) => {} // expected
            other => panic!("expected ContainerFailed, got: {:?}", other),
        }
    }
}

/// TS-07-E4: Get status for unknown adapter.
#[tokio::test]
async fn test_get_status_unknown() {
    let manager = make_manager(success_oci(), success_container());

    let result = manager.get_adapter_status("nonexistent-adapter").await;

    assert!(result.is_err(), "should return error for unknown adapter");
    match result.unwrap_err() {
        ManagerError::NotFound(_) => {} // expected
        other => panic!("expected NotFound, got: {:?}", other),
    }
}

/// TS-07-E5: Remove unknown adapter.
#[tokio::test]
async fn test_remove_unknown() {
    let manager = make_manager(success_oci(), success_container());

    let result = manager.remove_adapter("nonexistent-adapter").await;

    assert!(result.is_err(), "should return error for unknown adapter");
    match result.unwrap_err() {
        ManagerError::NotFound(_) => {} // expected
        other => panic!("expected NotFound, got: {:?}", other),
    }
}

/// TS-07-E6: Install already running adapter (same image).
#[tokio::test]
async fn test_install_already_running() {
    let manager = make_manager(success_oci(), success_container());
    let checksum = valid_checksum();

    // Install once
    manager
        .install_adapter("test-image:v1.0", &checksum)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Install same image again
    let result = manager
        .install_adapter("test-image:v1.0", &checksum)
        .await;

    assert!(result.is_err(), "should return error for duplicate install");
    match result.unwrap_err() {
        ManagerError::AlreadyExists(_) => {} // expected
        other => panic!("expected AlreadyExists, got: {:?}", other),
    }
}
