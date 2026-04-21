use std::sync::Arc;

use crate::adapter::AdapterState;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Wait for the named container to exit, then transition the adapter to
/// STOPPED (exit code 0) or ERROR (non-zero exit / wait failure).
///
/// Intended to be spawned as a background task after `podman run` succeeds.
pub async fn monitor_container<P: PodmanExecutor + Send + Sync + 'static>(
    adapter_id: String,
    _image_ref: String,
    state_mgr: Arc<StateManager>,
    podman: Arc<P>,
) {
    match podman.wait(&adapter_id).await {
        Ok(0) => {
            // Clean exit — transition to STOPPED (also records stopped_at for offload timer).
            if state_mgr
                .transition(&adapter_id, AdapterState::Stopped, None)
                .is_err()
            {
                // Adapter may have already been transitioned by another path (e.g. remove).
                // Use force_error as a fallback if normal transition is rejected.
                state_mgr.force_error(&adapter_id, "container exited cleanly but state transition failed");
            }
        }
        Ok(exit_code) => {
            // Non-zero exit — transition to ERROR.
            state_mgr.force_error(
                &adapter_id,
                &format!("container exited with non-zero code: {exit_code}"),
            );
        }
        Err(e) => {
            // podman wait failure — transition to ERROR.
            state_mgr.force_error(&adapter_id, &format!("podman wait failed: {}", e.message));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState};
    use crate::podman::{MockPodmanExecutor, PodmanError};
    use crate::state::StateManager;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn make_running_entry(adapter_id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: AdapterState::Running,
            job_id: "job-1".to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    fn make_state_mgr() -> Arc<StateManager> {
        let (tx, _rx) = broadcast::channel(128);
        Arc::new(StateManager::new(tx))
    }

    fn make_state_mgr_with_rx() -> (Arc<StateManager>, broadcast::Receiver<crate::adapter::AdapterStateEvent>) {
        let (tx, rx) = broadcast::channel(128);
        (Arc::new(StateManager::new(tx)), rx)
    }

    // TS-07-15: Container exits with non-zero code → adapter transitions to ERROR
    #[tokio::test]
    async fn test_container_exit_nonzero_error() {
        let sm = make_state_mgr();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        podman.set_wait_result(Ok(1)); // non-zero exit

        sm.create_adapter(make_running_entry(adapter_id, image_ref));
        monitor_container(
            adapter_id.to_string(),
            image_ref.to_string(),
            Arc::clone(&sm),
            Arc::clone(&podman),
        )
        .await;

        let adapter = sm.get_adapter(adapter_id).unwrap();
        assert_eq!(
            adapter.state,
            AdapterState::Error,
            "non-zero exit should transition to ERROR"
        );
    }

    // TS-07-16: Container exits with code 0 → adapter transitions to STOPPED
    // Also verifies stopped_at is set (integration point with offload timer).
    #[tokio::test]
    async fn test_container_exit_zero_stopped() {
        let sm = make_state_mgr();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        podman.set_wait_result(Ok(0)); // clean exit

        sm.create_adapter(make_running_entry(adapter_id, image_ref));
        monitor_container(
            adapter_id.to_string(),
            image_ref.to_string(),
            Arc::clone(&sm),
            Arc::clone(&podman),
        )
        .await;

        let adapter = sm.get_adapter(adapter_id).unwrap();
        assert_eq!(
            adapter.state,
            AdapterState::Stopped,
            "exit code 0 should transition to STOPPED"
        );
        assert!(
            adapter.stopped_at.is_some(),
            "stopped_at must be set so the offload timer can track inactivity"
        );
    }

    // TS-07-E16: Podman wait failure → adapter transitions to ERROR
    #[tokio::test]
    async fn test_podman_wait_failure_error() {
        let sm = make_state_mgr();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        podman.set_wait_result(Err(PodmanError::new("connection lost")));

        sm.create_adapter(make_running_entry(adapter_id, image_ref));
        monitor_container(
            adapter_id.to_string(),
            image_ref.to_string(),
            Arc::clone(&sm),
            Arc::clone(&podman),
        )
        .await;

        let adapter = sm.get_adapter(adapter_id).unwrap();
        assert_eq!(
            adapter.state,
            AdapterState::Error,
            "wait failure should transition to ERROR"
        );
    }

    // Verify RUNNING→STOPPED event is emitted to subscribers when container exits cleanly.
    // Ensures WatchAdapterStates subscribers see the transition (REQ-3.3, REQ-8.1).
    #[tokio::test]
    async fn test_container_exit_zero_emits_event() {
        let (sm, mut rx) = make_state_mgr_with_rx();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        podman.set_wait_result(Ok(0));
        sm.create_adapter(make_running_entry(adapter_id, image_ref));
        monitor_container(
            adapter_id.to_string(),
            image_ref.to_string(),
            Arc::clone(&sm),
            Arc::clone(&podman),
        )
        .await;

        let event = rx.try_recv().expect("should emit RUNNING→STOPPED event");
        assert_eq!(event.adapter_id, adapter_id);
        assert_eq!(event.old_state, AdapterState::Running);
        assert_eq!(event.new_state, AdapterState::Stopped);
        assert!(event.timestamp > 0);
    }

    // Verify RUNNING→ERROR event is emitted when container exits with non-zero code.
    #[tokio::test]
    async fn test_container_exit_nonzero_emits_error_event() {
        let (sm, mut rx) = make_state_mgr_with_rx();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        podman.set_wait_result(Ok(1));
        sm.create_adapter(make_running_entry(adapter_id, image_ref));
        monitor_container(
            adapter_id.to_string(),
            image_ref.to_string(),
            Arc::clone(&sm),
            Arc::clone(&podman),
        )
        .await;

        let event = rx.try_recv().expect("should emit RUNNING→ERROR event");
        assert_eq!(event.adapter_id, adapter_id);
        assert_eq!(event.old_state, AdapterState::Running);
        assert_eq!(event.new_state, AdapterState::Error);
    }

    // Integration test: monitor transitions to STOPPED → offload timer offloads.
    // Verifies the end-to-end flow between container monitor and offload timer.
    #[tokio::test]
    async fn test_monitor_to_offload_integration() {
        let (sm, mut rx) = make_state_mgr_with_rx();
        let podman = Arc::new(MockPodmanExecutor::new());
        let adapter_id = "adapter-a-v1";
        let image_ref = "registry.example.com/adapter-a:v1";

        // 1. Container exits cleanly → monitor transitions to STOPPED.
        podman.set_wait_result(Ok(0));
        sm.create_adapter(make_running_entry(adapter_id, image_ref));
        monitor_container(
            adapter_id.to_string(),
            image_ref.to_string(),
            Arc::clone(&sm),
            Arc::clone(&podman),
        )
        .await;

        let adapter = sm.get_adapter(adapter_id).unwrap();
        assert_eq!(adapter.state, AdapterState::Stopped);
        assert!(adapter.stopped_at.is_some());

        // 2. Wait past the inactivity timeout.
        let timeout = std::time::Duration::from_millis(1);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // 3. Offload timer fires → adapter should be offloaded.
        crate::offload::offload_expired(
            Arc::clone(&sm),
            Arc::clone(&podman),
            timeout,
        )
        .await;

        assert!(
            sm.get_adapter(adapter_id).is_none(),
            "adapter should be removed after offload"
        );
        assert!(
            podman.rm_calls().contains(&adapter_id.to_string()),
            "podman rm should be called during offload"
        );
        assert!(
            podman.rmi_calls().contains(&image_ref.to_string()),
            "podman rmi should be called during offload"
        );

        // 4. Verify the full event sequence: RUNNING→STOPPED, STOPPED→OFFLOADING.
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        assert!(
            events.len() >= 2,
            "expected at least 2 events (RUNNING→STOPPED, STOPPED→OFFLOADING), got {}",
            events.len()
        );
        assert_eq!(events[0].old_state, AdapterState::Running);
        assert_eq!(events[0].new_state, AdapterState::Stopped);
        assert_eq!(events[1].old_state, AdapterState::Stopped);
        assert_eq!(events[1].new_state, AdapterState::Offloading);
    }
}
