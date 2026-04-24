use std::sync::Arc;
use std::time::Duration;

use crate::adapter::AdapterState;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Runs the offload timer as a background loop.
///
/// Periodically checks for adapters that have been STOPPED longer than
/// `inactivity_timeout` and offloads them (removes container and image).
pub async fn run_offload_timer(
    state_mgr: Arc<StateManager>,
    podman: Arc<dyn PodmanExecutor>,
    inactivity_timeout: Duration,
    check_interval: Duration,
) {
    loop {
        tokio::time::sleep(check_interval).await;
        let candidates = state_mgr.get_offload_candidates(inactivity_timeout);
        for candidate in candidates {
            offload_adapter(
                &state_mgr,
                podman.as_ref(),
                &candidate.adapter_id,
                &candidate.image_ref,
            )
            .await;
        }
    }
}

/// Offloads a single adapter: transitions to OFFLOADING, removes container
/// and image, then removes from state.
pub async fn offload_adapter(
    state_mgr: &StateManager,
    podman: &dyn PodmanExecutor,
    adapter_id: &str,
    image_ref: &str,
) {
    // Transition to OFFLOADING
    if let Err(e) = state_mgr.transition(adapter_id, AdapterState::Offloading, None) {
        tracing::error!("failed to transition {adapter_id} to OFFLOADING: {e}");
        return;
    }

    // Remove container
    if let Err(e) = podman.rm(adapter_id).await {
        tracing::error!("failed to rm container {adapter_id}: {e}");
        let _ = state_mgr.transition(adapter_id, AdapterState::Error, Some(e.message));
        return;
    }

    // Remove image
    if let Err(e) = podman.rmi(image_ref).await {
        tracing::error!("failed to rmi image {image_ref}: {e}");
        let _ = state_mgr.transition(adapter_id, AdapterState::Error, Some(e.message));
        return;
    }

    // Remove from state
    if let Err(e) = state_mgr.remove_adapter(adapter_id) {
        tracing::error!("failed to remove adapter {adapter_id} from state: {e}");
    }
}
