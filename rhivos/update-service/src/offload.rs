use std::sync::Arc;
use std::time::Duration;

use tracing::{info, warn};

use crate::container::ContainerRuntime;
use crate::service::remove_adapter;
use crate::state::StateManager;

/// Default polling interval for the offload timer.
const OFFLOAD_POLL_INTERVAL_SECS: u64 = 60;

/// Spawn a background tokio task that periodically checks for adapters that
/// have been in STOPPED state longer than `inactivity_timeout_secs` and
/// removes them (offloads container + image, deletes state entry).
///
/// The task runs until the returned `JoinHandle` is aborted or the process
/// exits.
pub fn spawn_offload_timer(
    manager: Arc<StateManager>,
    runtime: Arc<dyn ContainerRuntime>,
    inactivity_timeout_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let poll_interval = Duration::from_secs(OFFLOAD_POLL_INTERVAL_SECS);
        loop {
            tokio::time::sleep(poll_interval).await;
            offload_expired(&manager, &runtime, inactivity_timeout_secs).await;
        }
    })
}

/// Check for expired stopped adapters and remove them.
///
/// This function is separated out so it can be called directly in tests
/// without spawning a task.
pub async fn offload_expired(
    manager: &Arc<StateManager>,
    runtime: &Arc<dyn ContainerRuntime>,
    inactivity_timeout_secs: u64,
) {
    let expired = manager.get_stopped_expired(inactivity_timeout_secs);
    for adapter_id in expired {
        info!(
            adapter_id = %adapter_id,
            timeout_secs = inactivity_timeout_secs,
            "auto-offloading adapter past inactivity threshold"
        );
        if let Err(e) = remove_adapter(
            Arc::clone(manager),
            Arc::clone(runtime),
            &adapter_id,
        )
        .await
        {
            warn!(
                adapter_id = %adapter_id,
                error = %e,
                "auto-offload failed for adapter"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::MockContainerRuntime;
    use crate::model::{now_unix, AdapterState};
    use crate::service::install_adapter;

    async fn install_and_stop(
        manager: &Arc<StateManager>,
        runtime: &Arc<MockContainerRuntime>,
        image_ref: &str,
        checksum: &str,
    ) -> String {
        let resp = install_adapter(
            Arc::clone(manager),
            Arc::clone(runtime) as Arc<dyn ContainerRuntime>,
            image_ref,
            checksum,
        )
        .await
        .expect("install should succeed");

        let adapter_id = resp.adapter_id.clone();
        // Transition to STOPPED
        manager
            .transition(&adapter_id, AdapterState::Stopped)
            .expect("should transition to STOPPED");
        adapter_id
    }

    // TS-07-16 (offload path): adapters past the timeout are removed by the offload loop.
    #[tokio::test]
    async fn test_offload_expired_removes_adapter() {
        let manager = Arc::new(StateManager::new());
        let runtime = Arc::new(
            MockContainerRuntime::new().with_digest("sha256:digest"),
        );

        let adapter_id = install_and_stop(
            &manager,
            &runtime,
            "registry.io/repo/adapter:v1",
            "sha256:digest",
        )
        .await;

        // Backdate stopped_at so it is past the timeout
        let past = now_unix() - 90;
        manager.set_stopped_at(&adapter_id, past);

        offload_expired(
            &manager,
            &(Arc::clone(&runtime) as Arc<dyn ContainerRuntime>),
            60, // 60 second timeout — adapter was stopped 90 s ago
        )
        .await;

        assert!(
            manager.get(&adapter_id).is_none(),
            "offloaded adapter must be removed from state"
        );
    }

    // Adapters within the timeout are NOT removed by the offload loop.
    #[tokio::test]
    async fn test_offload_skips_recent_adapter() {
        let manager = Arc::new(StateManager::new());
        let runtime = Arc::new(
            MockContainerRuntime::new().with_digest("sha256:digest"),
        );

        let adapter_id = install_and_stop(
            &manager,
            &runtime,
            "registry.io/repo/adapter:v1",
            "sha256:digest",
        )
        .await;

        // stopped_at is recent (10 seconds ago), timeout is 60 seconds — not expired
        let recent = now_unix() - 10;
        manager.set_stopped_at(&adapter_id, recent);

        offload_expired(
            &manager,
            &(Arc::clone(&runtime) as Arc<dyn ContainerRuntime>),
            60,
        )
        .await;

        assert!(
            manager.get(&adapter_id).is_some(),
            "adapter within timeout must NOT be offloaded"
        );
    }

    // TS-07-18 (offload events): STOPPED→OFFLOADING event emitted during auto-offload.
    #[tokio::test]
    async fn test_offload_emits_events() {
        let manager = Arc::new(StateManager::new());
        let runtime = Arc::new(
            MockContainerRuntime::new().with_digest("sha256:digest"),
        );

        let adapter_id = install_and_stop(
            &manager,
            &runtime,
            "registry.io/repo/adapter:v1",
            "sha256:digest",
        )
        .await;

        let past = now_unix() - 90;
        manager.set_stopped_at(&adapter_id, past);

        let mut rx = manager.subscribe();

        offload_expired(
            &manager,
            &(Arc::clone(&runtime) as Arc<dyn ContainerRuntime>),
            60,
        )
        .await;

        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }

        let has_offloading = events
            .iter()
            .any(|e| e.new_state == AdapterState::Offloading);
        assert!(has_offloading, "OFFLOADING event must be emitted during auto-offload");
    }
}
