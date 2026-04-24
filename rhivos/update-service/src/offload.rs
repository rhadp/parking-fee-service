use std::sync::Arc;
use std::time::Duration;

use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Runs the offload timer as a background loop.
///
/// Periodically checks for adapters that have been STOPPED longer than
/// `inactivity_timeout` and offloads them (removes container and image).
pub async fn run_offload_timer(
    _state_mgr: Arc<StateManager>,
    _podman: Arc<dyn PodmanExecutor>,
    _inactivity_timeout: Duration,
    _check_interval: Duration,
) {
    todo!("run_offload_timer not yet implemented")
}

/// Offloads a single adapter: transitions to OFFLOADING, removes container
/// and image, then removes from state.
pub async fn offload_adapter(
    _state_mgr: &StateManager,
    _podman: &dyn PodmanExecutor,
    _adapter_id: &str,
    _image_ref: &str,
) {
    todo!("offload_adapter not yet implemented")
}
