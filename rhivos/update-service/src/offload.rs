use std::sync::Arc;
use std::time::Duration;

use tokio::time;
use tracing::info;

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

    /// Compute the check interval: inactivity_timeout / 10, minimum 60 seconds.
    fn check_interval(&self) -> Duration {
        let interval_secs = self.inactivity_timeout_secs / 10;
        let interval_secs = interval_secs.max(60);
        Duration::from_secs(interval_secs)
    }

    /// Run the offload timer loop. This should be spawned as a tokio task.
    pub async fn run(&self) {
        let interval = self.check_interval();
        let timeout = Duration::from_secs(self.inactivity_timeout_secs);

        info!(
            interval_secs = interval.as_secs(),
            timeout_secs = self.inactivity_timeout_secs,
            "offload timer started"
        );

        let mut ticker = time::interval(interval);
        // The first tick fires immediately; skip it so we wait one interval first.
        ticker.tick().await;

        loop {
            ticker.tick().await;
            let offloaded = self.manager.offload_inactive_adapters(timeout).await;
            if !offloaded.is_empty() {
                info!(count = offloaded.len(), "offloaded inactive adapters");
            }
        }
    }
}

#[cfg(test)]
#[path = "offload_test.rs"]
mod offload_test;
