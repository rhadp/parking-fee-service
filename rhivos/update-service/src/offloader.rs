//! Offloader: background task that automatically offloads stopped adapters.
//!
//! Implements 04-REQ-6.1 (configurable inactivity timeout, default 24h),
//! 04-REQ-6.2 (stopped adapter offloaded after timeout, removed from list),
//! 04-REQ-6.3 (emits AdapterStateEvent during offloading transitions),
//! 04-REQ-6.E1 (re-install during OFFLOADING cancels offload, re-downloads).
//!
//! The offloader runs as a background tokio task. It periodically checks all
//! stopped adapters and transitions those whose `last_active` has exceeded
//! the configured `offload_timeout` through OFFLOADING -> UNKNOWN, then
//! removes them from the adapter list.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::adapter_manager::{AdapterManager, AdapterState};
use crate::container_runtime::ContainerRuntime;

/// Check interval for the offloader loop.
/// In tests, this can be short; in production it checks every 10 seconds.
const DEFAULT_CHECK_INTERVAL: Duration = Duration::from_secs(10);

/// Start the offloader background task.
///
/// The task periodically scans for adapters in the STOPPED state whose
/// `last_active` has exceeded the manager's `offload_timeout`. For each
/// such adapter it:
///
/// 1. Transitions to OFFLOADING (emitting a StateEvent)
/// 2. Removes container resources via the container runtime
/// 3. Transitions to UNKNOWN (emitting a StateEvent)
/// 4. Removes the adapter from the known adapters map
///
/// The task runs until the returned `tokio::task::JoinHandle` is aborted.
pub fn start_offloader<R: ContainerRuntime>(
    manager: Arc<Mutex<AdapterManager>>,
    runtime: Arc<R>,
    check_interval: Option<Duration>,
) -> tokio::task::JoinHandle<()> {
    let interval = check_interval.unwrap_or(DEFAULT_CHECK_INTERVAL);

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            offload_check(&manager, &runtime).await;
        }
    })
}

/// Single offload check iteration. Separated for testability.
async fn offload_check<R: ContainerRuntime>(
    manager: &Arc<Mutex<AdapterManager>>,
    runtime: &Arc<R>,
) {
    // Collect candidates under the lock, then release before doing I/O.
    let candidates: Vec<(String, Option<String>)> = {
        let mgr = manager.lock().await;
        let timeout = mgr.offload_timeout;

        mgr.stopped_adapters()
            .into_iter()
            .filter(|a| a.last_active.elapsed() > timeout)
            .map(|a| (a.adapter_id.clone(), a.container_id.clone()))
            .collect()
    };

    for (adapter_id, container_id) in candidates {
        // Step 1: Transition STOPPED -> OFFLOADING
        {
            let mut mgr = manager.lock().await;
            // Check the adapter is still STOPPED (could have been re-installed)
            match mgr.get_adapter(&adapter_id) {
                Some(record) if record.state == AdapterState::Stopped => {}
                _ => continue, // Adapter was removed or state changed, skip
            }
            if let Err(e) = mgr.transition(&adapter_id, AdapterState::Offloading) {
                eprintln!("offloader: failed to transition {} to OFFLOADING: {}", adapter_id, e);
                continue;
            }
        }

        // Step 2: Remove container resources (outside the lock)
        if let Some(ref cid) = container_id {
            if let Err(e) = runtime.remove_container(cid).await {
                eprintln!(
                    "offloader: failed to remove container {} for adapter {}: {}",
                    cid, adapter_id, e
                );
                // Continue with cleanup anyway
            }
        }

        // Step 3: Transition OFFLOADING -> UNKNOWN and remove from list
        {
            let mut mgr = manager.lock().await;
            // Check it's still in OFFLOADING (re-install could have happened)
            match mgr.get_adapter(&adapter_id) {
                Some(record) if record.state == AdapterState::Offloading => {}
                _ => continue, // Re-installed or removed, skip final cleanup
            }

            if let Err(e) = mgr.transition(&adapter_id, AdapterState::Unknown) {
                eprintln!(
                    "offloader: failed to transition {} to UNKNOWN: {}",
                    adapter_id, e
                );
                continue;
            }
            // Remove from the known adapters list
            let _ = mgr.remove_adapter(&adapter_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_runtime::MockContainerRuntime;

    #[tokio::test]
    async fn test_offload_check_offloads_stopped_adapter() {
        // Create a manager with a very short timeout (0 seconds)
        let mut mgr = AdapterManager::new(Duration::from_secs(0));
        mgr.install_adapter(
            "adapter-1".to_string(),
            "test:v1".to_string(),
            "checksum".to_string(),
        )
        .unwrap();
        // Transition through lifecycle: DOWNLOADING -> INSTALLING -> RUNNING -> STOPPED
        mgr.transition("adapter-1", AdapterState::Installing).unwrap();
        mgr.transition("adapter-1", AdapterState::Running).unwrap();
        mgr.transition("adapter-1", AdapterState::Stopped).unwrap();

        let manager = Arc::new(Mutex::new(mgr));
        let runtime = Arc::new(MockContainerRuntime::new());

        // Sleep briefly so elapsed > 0
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Run a single offload check
        offload_check(&manager, &runtime).await;

        // Adapter should have been offloaded and removed
        let mgr = manager.lock().await;
        assert!(
            mgr.list_adapters().is_empty(),
            "adapter should be removed after offloading"
        );
    }

    #[tokio::test]
    async fn test_offload_check_skips_running_adapter() {
        // Create a manager with a 0-second timeout
        let mut mgr = AdapterManager::new(Duration::from_secs(0));
        mgr.install_adapter(
            "adapter-1".to_string(),
            "test:v1".to_string(),
            "checksum".to_string(),
        )
        .unwrap();
        mgr.transition("adapter-1", AdapterState::Installing).unwrap();
        mgr.transition("adapter-1", AdapterState::Running).unwrap();

        let manager = Arc::new(Mutex::new(mgr));
        let runtime = Arc::new(MockContainerRuntime::new());

        tokio::time::sleep(Duration::from_millis(10)).await;

        offload_check(&manager, &runtime).await;

        // Running adapter should NOT be offloaded
        let mgr = manager.lock().await;
        assert_eq!(mgr.list_adapters().len(), 1);
        assert_eq!(
            mgr.get_adapter("adapter-1").unwrap().state,
            AdapterState::Running
        );
    }

    #[tokio::test]
    async fn test_offload_check_respects_timeout() {
        // Timeout of 10 seconds — adapter stopped for 0 seconds should NOT be offloaded
        let mut mgr = AdapterManager::new(Duration::from_secs(10));
        mgr.install_adapter(
            "adapter-1".to_string(),
            "test:v1".to_string(),
            "checksum".to_string(),
        )
        .unwrap();
        mgr.transition("adapter-1", AdapterState::Installing).unwrap();
        mgr.transition("adapter-1", AdapterState::Running).unwrap();
        mgr.transition("adapter-1", AdapterState::Stopped).unwrap();

        let manager = Arc::new(Mutex::new(mgr));
        let runtime = Arc::new(MockContainerRuntime::new());

        // No sleep — adapter just stopped
        offload_check(&manager, &runtime).await;

        // Should NOT be offloaded yet
        let mgr = manager.lock().await;
        assert_eq!(mgr.list_adapters().len(), 1);
        assert_eq!(
            mgr.get_adapter("adapter-1").unwrap().state,
            AdapterState::Stopped
        );
    }

    #[tokio::test]
    async fn test_offload_emits_events() {
        let mut mgr = AdapterManager::new(Duration::from_secs(0));
        let mut rx = mgr.subscribe();

        mgr.install_adapter(
            "adapter-1".to_string(),
            "test:v1".to_string(),
            "checksum".to_string(),
        )
        .unwrap();
        mgr.transition("adapter-1", AdapterState::Installing).unwrap();
        mgr.transition("adapter-1", AdapterState::Running).unwrap();
        mgr.transition("adapter-1", AdapterState::Stopped).unwrap();

        // Drain existing events
        while rx.try_recv().is_ok() {}

        let manager = Arc::new(Mutex::new(mgr));
        let runtime = Arc::new(MockContainerRuntime::new());

        tokio::time::sleep(Duration::from_millis(10)).await;

        offload_check(&manager, &runtime).await;

        // Should have received STOPPED -> OFFLOADING and OFFLOADING -> UNKNOWN events
        let event1 = rx.try_recv().expect("should receive OFFLOADING event");
        assert_eq!(event1.new_state, AdapterState::Offloading);

        let event2 = rx.try_recv().expect("should receive UNKNOWN event");
        assert_eq!(event2.new_state, AdapterState::Unknown);
    }
}
