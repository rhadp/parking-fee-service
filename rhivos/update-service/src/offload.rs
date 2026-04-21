use std::sync::Arc;
use std::time::Duration;

use crate::adapter::AdapterState;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Iterate over STOPPED adapters whose inactivity timeout has elapsed,
/// transition them to OFFLOADING, and clean up via `podman rm` + `podman rmi`.
/// On any cleanup failure the adapter is transitioned to ERROR and left in state.
pub async fn offload_expired<P: PodmanExecutor + Send + Sync + 'static>(
    state_mgr: Arc<StateManager>,
    podman: Arc<P>,
    timeout: Duration,
) {
    let candidates = state_mgr.get_offload_candidates(timeout);

    for candidate in candidates {
        let adapter_id = candidate.adapter_id.clone();
        let image_ref = candidate.image_ref.clone();

        // Transition to OFFLOADING and emit the state event.
        if state_mgr
            .transition(&adapter_id, AdapterState::Offloading, None)
            .is_err()
        {
            // Adapter was concurrently modified; skip.
            continue;
        }

        // Remove container.
        if let Err(e) = podman.rm(&adapter_id).await {
            state_mgr.force_error(&adapter_id, &e.message);
            continue;
        }

        // Remove image.
        if let Err(e) = podman.rmi(&image_ref).await {
            state_mgr.force_error(&adapter_id, &e.message);
            continue;
        }

        // All cleanup succeeded — remove from in-memory state entirely.
        let _ = state_mgr.remove_adapter(&adapter_id);
    }
}

/// Spawn a background task that calls `offload_expired` every `interval_secs` seconds.
pub fn spawn_offload_timer<P: PodmanExecutor + Send + Sync + 'static>(
    state_mgr: Arc<StateManager>,
    podman: Arc<P>,
    timeout: Duration,
    interval_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            offload_expired(Arc::clone(&state_mgr), Arc::clone(&podman), timeout).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState};
    use crate::podman::MockPodmanExecutor;
    use crate::state::StateManager;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn make_stopped_entry(adapter_id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: AdapterState::Stopped,
            job_id: "job-1".to_string(),
            stopped_at: Some(std::time::Instant::now()),
            error_message: None,
        }
    }

    fn make_state_mgr() -> (Arc<StateManager>, broadcast::Receiver<crate::adapter::AdapterStateEvent>) {
        let (tx, rx) = broadcast::channel(128);
        (Arc::new(StateManager::new(tx)), rx)
    }

    // TS-07-13: STOPPED adapter is offloaded after timeout
    // Verifies: transition to OFFLOADING, rm + rmi called, adapter removed,
    // and STOPPED→OFFLOADING event delivered to subscribers (REQ-6.4).
    #[tokio::test]
    async fn test_offload_after_timeout() {
        let (sm, mut rx) = make_state_mgr();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        sm.create_adapter(make_stopped_entry(adapter_id, image_ref));

        // Use a very short timeout so the adapter is eligible
        let timeout = Duration::from_millis(1);
        tokio::time::sleep(Duration::from_millis(10)).await; // ensure stopped_at is in the past

        offload_expired(Arc::clone(&sm), Arc::clone(&podman), timeout).await;

        assert!(
            sm.get_adapter(adapter_id).is_none(),
            "adapter should be removed after offload"
        );
        assert!(
            podman.rm_calls().contains(&adapter_id.to_string()),
            "podman rm should be called"
        );
        assert!(
            podman.rmi_calls().contains(&image_ref.to_string()),
            "podman rmi should be called"
        );

        // Verify STOPPED→OFFLOADING event was emitted (REQ-6.4)
        let mut found_offloading_event = false;
        while let Ok(event) = rx.try_recv() {
            if event.adapter_id == adapter_id
                && event.old_state == AdapterState::Stopped
                && event.new_state == AdapterState::Offloading
            {
                found_offloading_event = true;
            }
        }
        assert!(
            found_offloading_event,
            "STOPPED→OFFLOADING event should be delivered to subscribers"
        );
    }

    // TS-07-E12: Offload cleanup failure transitions adapter to ERROR
    #[tokio::test]
    async fn test_offload_failure_error() {
        use crate::podman::PodmanError;

        let (sm, _rx) = make_state_mgr();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        podman.set_rm_result(Ok(()));
        podman.set_rmi_result(Err(PodmanError::new("image in use")));

        sm.create_adapter(make_stopped_entry(adapter_id, image_ref));

        let timeout = Duration::from_millis(1);
        tokio::time::sleep(Duration::from_millis(10)).await;

        offload_expired(Arc::clone(&sm), Arc::clone(&podman), timeout).await;

        let adapter = sm.get_adapter(adapter_id).expect("adapter should still exist on failure");
        assert_eq!(
            adapter.state,
            AdapterState::Error,
            "adapter should be in ERROR state after offload failure"
        );
    }

    // TS-07-P6: Offload timing correctness property test
    // For any inactivity timeout T (2..10 seconds), an adapter that just
    // entered STOPPED state is NOT offloaded when offload_expired runs
    // immediately (elapsed time ≈ 0 < T).
    #[test]
    #[ignore]
    fn proptest_offload_timing_correctness() {
        use proptest::prelude::*;

        proptest!(|(timeout_secs in 2u64..10)| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let result: Result<(), String> = rt.block_on(async {
                let (tx, _rx) = broadcast::channel(128);
                let state_mgr = Arc::new(StateManager::new(tx));
                let podman = Arc::new(MockPodmanExecutor::new());
                let adapter_id = "adapter-a-v1";
                let image_ref = "registry.example.com/adapter-a:v1";

                // Create adapter with stopped_at = now.
                state_mgr.create_adapter(make_stopped_entry(adapter_id, image_ref));

                let timeout = Duration::from_secs(timeout_secs);
                // Call offload immediately — adapter was JUST stopped.
                offload_expired(
                    Arc::clone(&state_mgr),
                    Arc::clone(&podman),
                    timeout,
                )
                .await;

                let adapter = state_mgr.get_adapter(adapter_id);
                if adapter.is_none() {
                    return Err(format!(
                        "Adapter was offloaded immediately, but timeout is {}s",
                        timeout_secs
                    ));
                }
                if adapter.unwrap().state != AdapterState::Stopped {
                    return Err(
                        "Adapter should still be STOPPED before timeout".to_string(),
                    );
                }
                Ok(())
            });
            prop_assert!(result.is_ok(), "{}", result.err().unwrap_or_default());
        });
    }
}
