use std::sync::Arc;
use crate::manager::AdapterManager;

/// Background task that periodically checks for stopped adapters
/// and offloads them after the configured inactivity timeout.
pub struct OffloadTimer {
    pub manager: Arc<AdapterManager>,
    pub inactivity_timeout_secs: u64,
}

impl OffloadTimer {
    pub fn new(manager: Arc<AdapterManager>, inactivity_timeout_secs: u64) -> Self {
        Self {
            manager,
            inactivity_timeout_secs,
        }
    }

    /// Run the offload timer loop. This should be spawned as a tokio task.
    pub async fn run(&self) {
        // Stub: not implemented. Implementation in task group 6.
    }
}

#[cfg(test)]
#[path = "offload_test.rs"]
mod offload_test;
