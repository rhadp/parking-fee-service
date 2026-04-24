use std::sync::Arc;

use crate::adapter::AdapterState;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Monitors a running container by calling `podman wait` and updating
/// the adapter state based on the exit code.
///
/// - Exit code 0 → transition to STOPPED
/// - Exit code non-zero → transition to ERROR
/// - Wait failure → transition to ERROR
pub async fn monitor_container(
    state_mgr: Arc<StateManager>,
    podman: Arc<dyn PodmanExecutor>,
    adapter_id: String,
) {
    match podman.wait(&adapter_id).await {
        Ok(0) => {
            let _ = state_mgr.transition(&adapter_id, AdapterState::Stopped, None);
        }
        Ok(code) => {
            let _ = state_mgr.transition(
                &adapter_id,
                AdapterState::Error,
                Some(format!("container exited with code {code}")),
            );
        }
        Err(e) => {
            let _ = state_mgr.transition(
                &adapter_id,
                AdapterState::Error,
                Some(e.message),
            );
        }
    }
}
