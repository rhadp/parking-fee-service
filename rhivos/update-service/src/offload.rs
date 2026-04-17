use std::sync::Arc;
use std::time::Duration;

use crate::adapter::AdapterState;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Spawn a background task that periodically offloads unused STOPPED adapters.
///
/// The task fires every `poll_interval` (waiting before the first check) and calls
/// `offload_expired` each time. Returns the `JoinHandle` so the caller can abort it
/// on shutdown.
pub async fn spawn_offload_timer<P: PodmanExecutor + Send + Sync + 'static>(
    state: Arc<StateManager>,
    podman: Arc<P>,
    inactivity_timeout: Duration,
    poll_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(poll_interval).await;
            offload_expired(state.clone(), podman.clone(), inactivity_timeout).await;
        }
    })
}

/// Offload all adapters that have been STOPPED longer than `inactivity_timeout`.
///
/// For each expired adapter:
/// 1. Transition to OFFLOADING (emits event to subscribers).
/// 2. Remove the container via `podman rm`.
/// 3. Remove the image via `podman rmi`.
/// 4. On success: remove the adapter from in-memory state.
/// 5. On any failure: leave the adapter in ERROR state.
pub async fn offload_expired<P: PodmanExecutor + Send + Sync + 'static>(
    state: Arc<StateManager>,
    podman: Arc<P>,
    inactivity_timeout: Duration,
) {
    let candidates = state.get_offload_candidates(inactivity_timeout);
    for entry in candidates {
        let adapter_id = entry.adapter_id.clone();
        let image_ref = entry.image_ref.clone();

        // Transition to OFFLOADING and emit the state event.
        if state
            .transition(&adapter_id, AdapterState::Offloading, None)
            .is_err()
        {
            // Adapter was removed between get_offload_candidates and now; skip.
            continue;
        }

        // Remove the container.
        if let Err(e) = podman.rm(&adapter_id).await {
            state.force_error(
                &adapter_id,
                &format!("offload rm failed: {}", e.message),
            );
            continue;
        }

        // Remove the image.
        if let Err(e) = podman.rmi(&image_ref).await {
            state.force_error(
                &adapter_id,
                &format!("offload rmi failed: {}", e.message),
            );
            continue;
        }

        // Both commands succeeded — drop the adapter from in-memory state entirely.
        let _ = state.remove_adapter(&adapter_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState};
    use crate::podman::{MockPodmanExecutor, PodmanError};
    use crate::state::StateManager;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::broadcast;

    fn make_stopped_entry(id: &str, image_ref: &str, stopped_age: Duration) -> AdapterEntry {
        AdapterEntry {
            adapter_id: id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: AdapterState::Stopped,
            job_id: uuid::Uuid::new_v4().to_string(),
            stopped_at: Some(Instant::now() - stopped_age),
            error_message: None,
        }
    }

    // TS-07-13: Offload timer triggers after inactivity
    #[tokio::test]
    async fn test_offload_after_timeout() {
        let (tx, mut rx) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx));
        let mock = Arc::new(MockPodmanExecutor::new());
        let mock_clone = mock.clone();

        // Create a stopped adapter that has been stopped for 2 seconds
        let entry = make_stopped_entry(
            "parkhaus-munich-v1.0.0",
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
            Duration::from_secs(2),
        );
        state.create_adapter(entry);

        // Offload with 1 second timeout - adapter should be expired
        offload_expired(state.clone(), mock, Duration::from_secs(1)).await;

        // Adapter should be removed from state
        assert!(state.get_adapter("parkhaus-munich-v1.0.0").is_none());
        // rm and rmi should have been called
        assert!(mock_clone
            .rm_calls()
            .contains(&"parkhaus-munich-v1.0.0".to_string()));
        assert!(mock_clone
            .rmi_calls()
            .contains(&"us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0".to_string()));

        // Should have emitted STOPPED->OFFLOADING event
        let event = rx.try_recv().unwrap();
        assert_eq!(event.old_state, AdapterState::Stopped);
        assert_eq!(event.new_state, AdapterState::Offloading);
    }

    // TS-07-E12: Offload cleanup failure transitions to ERROR
    #[tokio::test]
    async fn test_offload_failure_error() {
        let (tx, _rx) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx));
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_rm_result(Ok(()));
        mock.set_rmi_result(Err(PodmanError::new("image in use")));

        // Create a stopped adapter that has been stopped for 2 seconds
        let entry = make_stopped_entry(
            "parkhaus-munich-v1.0.0",
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
            Duration::from_secs(2),
        );
        state.create_adapter(entry);

        // Offload with 1 second timeout - should fail and transition to ERROR
        offload_expired(state.clone(), mock, Duration::from_secs(1)).await;

        // Adapter should still be in state (not removed), but in ERROR state
        let adapter = state.get_adapter("parkhaus-munich-v1.0.0").unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
    }
}

// TS-07-P6: Offload timing correctness — offloading must not occur before timeout
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState};
    use crate::podman::MockPodmanExecutor;
    use proptest::prelude::*;
    use std::time::Instant;

    fn make_stopped_entry_recent(
        id: &str,
        image_ref: &str,
        stopped_secs_ago: u64,
    ) -> AdapterEntry {
        let stopped_at = Instant::now()
            .checked_sub(std::time::Duration::from_secs(stopped_secs_ago))
            .unwrap_or_else(Instant::now);
        AdapterEntry {
            adapter_id: id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: AdapterState::Stopped,
            job_id: uuid::Uuid::new_v4().to_string(),
            stopped_at: Some(stopped_at),
            error_message: None,
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(10))]

        // TS-07-P6: Adapters stopped less than timeout_secs ago must NOT be offloaded
        #[test]
        #[ignore = "proptest: run with --include-ignored"]
        fn proptest_offload_timing_correctness(
            timeout_secs in 3u64..10,
            stopped_secs_ago in 0u64..2,
        ) {
            // stopped_secs_ago < timeout_secs, so adapter should NOT be offloaded
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            // Collect results from async context, assert outside
            let (still_exists, state_opt) = rt.block_on(async {
                let (tx, _rx) = tokio::sync::broadcast::channel(100);
                let state = Arc::new(StateManager::new(tx));
                let mock = Arc::new(MockPodmanExecutor::new());
                mock.set_rm_result(Ok(()));
                mock.set_rmi_result(Ok(()));

                let adapter_id = "test-adapter-v1";
                let entry = make_stopped_entry_recent(
                    adapter_id,
                    "example.com/test-adapter:v1",
                    stopped_secs_ago,
                );
                state.create_adapter(entry);

                // Offload with a longer timeout — adapter should NOT be removed
                offload_expired(
                    state.clone(),
                    mock,
                    std::time::Duration::from_secs(timeout_secs),
                )
                .await;

                let adapter = state.get_adapter(adapter_id);
                let still_exists = adapter.is_some();
                let state_opt = adapter.map(|a| a.state);
                (still_exists, state_opt)
            });
            prop_assert!(
                still_exists,
                "Adapter offloaded prematurely (stopped {}s ago, timeout {}s)",
                stopped_secs_ago,
                timeout_secs
            );
            prop_assert_eq!(state_opt, Some(AdapterState::Stopped));
        }
    }
}
