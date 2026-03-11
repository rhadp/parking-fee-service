use std::sync::Arc;
use std::time::Duration;

use crate::container::MockContainerRuntime;
use crate::manager::AdapterManager;
use crate::oci::{MockOciPuller, PullResult};
use crate::state::AdapterState;

use super::OffloadTimer;

/// Compute the expected SHA-256 checksum of a digest string for test fixtures.
fn compute_test_checksum(digest: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(digest.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

/// Helper: create mock OCI puller that returns a successful pull.
fn mock_oci_success(digest: &str) -> MockOciPuller {
    let digest = digest.to_string();
    let mut mock = MockOciPuller::new();
    mock.expect_pull_image()
        .returning(move |_| Ok(PullResult { digest: digest.clone() }));
    mock.expect_remove_image().returning(|_| Ok(()));
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

// TS-07-8: Offloading after inactivity
#[tokio::test]
async fn test_offload_after_inactivity() {
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
    let manager = Arc::new(AdapterManager::new(Arc::new(oci), Arc::new(container)));

    // Install adapter A
    let result_a = manager
        .install_adapter("test-registry/image-a:v1.0", &checksum_a)
        .await
        .expect("install adapter A should succeed");
    let adapter_a_id = result_a.adapter_id.clone();

    // Install adapter B (stops A, so A becomes STOPPED)
    manager
        .install_adapter("test-registry/image-b:v1.0", &checksum_b)
        .await
        .expect("install adapter B should succeed");

    // Verify A is STOPPED
    let status_a = manager.get_adapter_status(&adapter_a_id).await.unwrap();
    assert_eq!(status_a.state, AdapterState::Stopped, "adapter A should be STOPPED");

    // Now offload with zero timeout — adapter A should be offloaded immediately
    let offloaded = manager
        .offload_inactive_adapters(Duration::from_secs(0))
        .await;
    assert!(
        offloaded.contains(&adapter_a_id),
        "adapter A should be offloaded, offloaded: {:?}",
        offloaded
    );

    // Adapter A should no longer be listed
    let adapters = manager.list_adapters().await;
    let found = adapters.iter().any(|a| a.adapter_id == adapter_a_id);
    assert!(!found, "offloaded adapter should not be listed");

    // Adapter B should still be running
    let running_count = adapters.iter().filter(|a| a.state == AdapterState::Running).count();
    assert_eq!(running_count, 1, "adapter B should still be running");
}

// Test that running adapters are NOT offloaded
#[tokio::test]
async fn test_running_adapter_not_offloaded() {
    let digest = "sha256:abc123";
    let checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let manager = Arc::new(AdapterManager::new(Arc::new(oci), Arc::new(container)));

    manager
        .install_adapter("test-registry/image:v1.0", &checksum)
        .await
        .expect("install should succeed");

    // Try to offload with zero timeout — should NOT offload running adapter
    let offloaded = manager
        .offload_inactive_adapters(Duration::from_secs(0))
        .await;
    assert!(offloaded.is_empty(), "running adapter should not be offloaded");

    let adapters = manager.list_adapters().await;
    assert_eq!(adapters.len(), 1, "adapter should still exist");
    assert_eq!(adapters[0].state, AdapterState::Running, "adapter should still be RUNNING");
}

// Test that the offload timer computes the check interval correctly
#[test]
fn test_check_interval_minimum_60_seconds() {
    let oci = MockOciPuller::new();
    let container = MockContainerRuntime::new();
    let manager = Arc::new(AdapterManager::new(Arc::new(oci), Arc::new(container)));

    // With a small timeout, interval should be at least 60 seconds
    let timer = OffloadTimer::new(manager.clone(), 100);
    assert_eq!(timer.check_interval(), Duration::from_secs(60));

    // With a large timeout, interval should be timeout / 10
    let timer = OffloadTimer::new(manager, 36000);
    assert_eq!(timer.check_interval(), Duration::from_secs(3600));
}

// Test that stopped adapter is NOT offloaded when timeout hasn't elapsed
#[tokio::test]
async fn test_stopped_adapter_not_offloaded_before_timeout() {
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
    let manager = Arc::new(AdapterManager::new(Arc::new(oci), Arc::new(container)));

    // Install A then B (A becomes STOPPED)
    manager
        .install_adapter("test-registry/image-a:v1.0", &checksum_a)
        .await
        .expect("install A");
    manager
        .install_adapter("test-registry/image-b:v1.0", &checksum_b)
        .await
        .expect("install B");

    // Try to offload with a very long timeout — should NOT offload
    let offloaded = manager
        .offload_inactive_adapters(Duration::from_secs(86400))
        .await;
    assert!(offloaded.is_empty(), "adapter should not be offloaded before timeout");

    let adapters = manager.list_adapters().await;
    assert_eq!(adapters.len(), 2, "both adapters should still exist");
}
