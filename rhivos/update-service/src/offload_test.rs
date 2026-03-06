use std::sync::Arc;

use crate::container::MockContainerRuntime;
use crate::manager::AdapterManager;
use crate::oci::{MockOciPuller, PullResult};
use crate::state::AdapterState;

use super::OffloadTimer;

/// Helper to create a mock OCI puller that succeeds.
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

/// Valid checksum matching the test digest.
fn valid_checksum() -> String {
    use sha2::{Digest, Sha256};
    let digest_str =
        "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let hash = Sha256::digest(digest_str.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

/// TS-07-8: Offloading after inactivity.
///
/// A stopped adapter is automatically offloaded after the configured inactivity
/// timeout expires.
#[tokio::test(start_paused = true)]
async fn test_offloading_after_inactivity() {
    let manager = Arc::new(AdapterManager::new(
        Arc::new(success_oci()),
        Arc::new(success_container()),
    ));
    let checksum = valid_checksum();

    // Install adapter A
    let result_a = manager.install_adapter("image-a:v1.0", &checksum).await;
    assert!(result_a.is_ok());
    let adapter_a_id = result_a.unwrap().adapter_id;

    // Install adapter B (stops A, making A STOPPED)
    let result_b = manager.install_adapter("image-b:v1.0", &checksum).await;
    assert!(result_b.is_ok());

    // Verify adapter A is STOPPED
    let status_a = manager.get_adapter_status(&adapter_a_id).await;
    assert!(status_a.is_ok());
    assert_eq!(status_a.unwrap().state, AdapterState::Stopped);

    // Create offload timer with 1 second timeout
    let timer = OffloadTimer::new(Arc::clone(&manager), 1);

    // Advance time past the inactivity timeout + check interval (min 60s)
    // The check interval is max(timeout/10, 60s) = 60s for 1s timeout
    tokio::time::advance(std::time::Duration::from_secs(61)).await;

    // Run one offload check manually instead of relying on the timer loop
    let offloaded = manager
        .offload_inactive_adapters(timer.inactivity_timeout)
        .await;

    assert!(
        offloaded.contains(&adapter_a_id),
        "adapter A should have been offloaded, offloaded: {:?}",
        offloaded
    );

    // Adapter A should no longer be listed
    let adapters = manager.list_adapters().await;
    let found_a = adapters.iter().any(|a| a.adapter_id == adapter_a_id);
    assert!(
        !found_a,
        "offloaded adapter A should not appear in adapter list"
    );

    // Adapter B should still be running
    let running = adapters
        .iter()
        .filter(|a| a.state == AdapterState::Running)
        .count();
    assert_eq!(running, 1, "adapter B should still be running");
}

/// TS-07-8 (variant): Adapters NOT past timeout should NOT be offloaded.
#[tokio::test(start_paused = true)]
async fn test_no_offload_before_timeout() {
    let manager = Arc::new(AdapterManager::new(
        Arc::new(success_oci()),
        Arc::new(success_container()),
    ));
    let checksum = valid_checksum();

    // Install adapter A, then B (stops A)
    let result_a = manager.install_adapter("image-a:v1.0", &checksum).await;
    assert!(result_a.is_ok());
    let adapter_a_id = result_a.unwrap().adapter_id;

    manager
        .install_adapter("image-b:v1.0", &checksum)
        .await
        .unwrap();

    // Only advance 500ms (timeout is 10s)
    let timeout = std::time::Duration::from_secs(10);
    tokio::time::advance(std::time::Duration::from_millis(500)).await;

    let offloaded = manager.offload_inactive_adapters(timeout).await;
    assert!(
        offloaded.is_empty(),
        "should not offload before timeout expires"
    );

    // Adapter A should still be listed
    let adapters = manager.list_adapters().await;
    let found_a = adapters.iter().any(|a| a.adapter_id == adapter_a_id);
    assert!(found_a, "adapter A should still be listed");
}

/// TS-07-8 (variant): Running adapters should NOT be offloaded even if old.
#[tokio::test(start_paused = true)]
async fn test_no_offload_running_adapter() {
    let manager = Arc::new(AdapterManager::new(
        Arc::new(success_oci()),
        Arc::new(success_container()),
    ));
    let checksum = valid_checksum();

    // Install one adapter (it becomes RUNNING)
    let result = manager
        .install_adapter("test-image:v1.0", &checksum)
        .await
        .unwrap();

    // Advance time well past any timeout
    tokio::time::advance(std::time::Duration::from_secs(100_000)).await;

    let offloaded = manager
        .offload_inactive_adapters(std::time::Duration::from_secs(1))
        .await;
    assert!(
        offloaded.is_empty(),
        "running adapter should not be offloaded"
    );

    let status = manager.get_adapter_status(&result.adapter_id).await;
    assert_eq!(status.unwrap().state, AdapterState::Running);
}

/// TS-07-8 (variant): Verify the timer loop actually fires and offloads.
#[tokio::test(start_paused = true)]
async fn test_offload_timer_loop() {
    let manager = Arc::new(AdapterManager::new(
        Arc::new(success_oci()),
        Arc::new(success_container()),
    ));
    let checksum = valid_checksum();

    // Install A then B (stops A)
    let result_a = manager.install_adapter("image-a:v1.0", &checksum).await;
    assert!(result_a.is_ok());
    let adapter_a_id = result_a.unwrap().adapter_id;

    manager
        .install_adapter("image-b:v1.0", &checksum)
        .await
        .unwrap();

    // Use a 10-second timeout; check interval = max(1s, 60s) = 60s
    let timer = OffloadTimer::new(Arc::clone(&manager), 10);

    // Spawn the timer
    let timer_handle = tokio::spawn(async move {
        timer.run().await;
    });

    // We need to advance time past the check interval (60s) and yield
    // multiple times to allow the spawned task to wake, run, and complete
    // its offload operations.
    for _ in 0..10 {
        tokio::time::advance(std::time::Duration::from_secs(7)).await;
        tokio::task::yield_now().await;
    }

    // Adapter A should have been offloaded
    let adapters = manager.list_adapters().await;
    let found_a = adapters.iter().any(|a| a.adapter_id == adapter_a_id);
    assert!(
        !found_a,
        "adapter A should have been offloaded by timer loop"
    );

    timer_handle.abort();
}

/// TS-07-P2: Single adapter invariant property test.
///
/// After any sequence of InstallAdapter and RemoveAdapter operations,
/// at most one adapter is in RUNNING state.
#[tokio::test]
async fn test_single_adapter_invariant() {
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

    let manager = Arc::new(AdapterManager::new(Arc::new(oci), Arc::new(container)));
    let checksum = valid_checksum();

    // Images to cycle through
    let images = [
        "image-alpha:v1",
        "image-beta:v1",
        "image-gamma:v1",
        "image-delta:v1",
        "image-epsilon:v1",
    ];

    // Sequence of install/remove operations
    let mut installed_ids: Vec<String> = Vec::new();

    for (i, img) in images.iter().enumerate() {
        // Install adapter
        let result = manager.install_adapter(img, &checksum).await;
        assert!(result.is_ok(), "install #{i} should succeed");
        installed_ids.push(result.unwrap().adapter_id);

        // After each install, assert single-adapter invariant
        let adapters = manager.list_adapters().await;
        let running_count = adapters
            .iter()
            .filter(|a| a.state == AdapterState::Running)
            .count();
        assert!(
            running_count <= 1,
            "invariant violated after install #{i}: {running_count} adapters RUNNING"
        );
    }

    // Now remove some and install again, checking invariant each time
    for id in &installed_ids[..3] {
        let _ = manager.remove_adapter(id).await;

        let adapters = manager.list_adapters().await;
        let running_count = adapters
            .iter()
            .filter(|a| a.state == AdapterState::Running)
            .count();
        assert!(
            running_count <= 1,
            "invariant violated after remove: {running_count} adapters RUNNING"
        );
    }

    // Re-install some
    for img in &images[..3] {
        let result = manager.install_adapter(img, &checksum).await;
        assert!(result.is_ok());

        let adapters = manager.list_adapters().await;
        let running_count = adapters
            .iter()
            .filter(|a| a.state == AdapterState::Running)
            .count();
        assert!(
            running_count <= 1,
            "invariant violated after re-install: {running_count} adapters RUNNING"
        );
    }

    // Final assertion
    let adapters = manager.list_adapters().await;
    let running_count = adapters
        .iter()
        .filter(|a| a.state == AdapterState::Running)
        .count();
    assert!(
        running_count <= 1,
        "final invariant check: {running_count} adapters RUNNING"
    );
}
