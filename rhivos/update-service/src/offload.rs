use std::sync::Arc;

use crate::manager::AdapterManager;

/// Background task that offloads stopped adapters after an inactivity timeout.
pub struct OffloadTimer {
    pub(crate) _manager: Arc<AdapterManager>,
    pub(crate) _inactivity_timeout_secs: u64,
}

impl OffloadTimer {
    pub fn new(manager: Arc<AdapterManager>, inactivity_timeout_secs: u64) -> Self {
        Self {
            _manager: manager,
            _inactivity_timeout_secs: inactivity_timeout_secs,
        }
    }

    /// Run the offload timer loop. Intended to be spawned as a background task.
    pub async fn run(&self) {
        // Stub: not yet implemented
        todo!("offload timer not yet implemented")
    }
}

#[cfg(test)]
#[path = "offload_test.rs"]
mod tests;
