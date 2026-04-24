use std::sync::Arc;

use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Monitors a running container by calling `podman wait` and updating
/// the adapter state based on the exit code.
///
/// - Exit code 0 → transition to STOPPED
/// - Exit code non-zero → transition to ERROR
/// - Wait failure → transition to ERROR
pub async fn monitor_container(
    _state_mgr: Arc<StateManager>,
    _podman: Arc<dyn PodmanExecutor>,
    _adapter_id: String,
) {
    todo!("monitor_container not yet implemented")
}
