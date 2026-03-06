use std::sync::Arc;

use tokio::time::Duration;
use tracing::info;

use crate::manager::AdapterManager;

/// Background task that offloads stopped adapters after an inactivity timeout.
pub struct OffloadTimer {
    pub(crate) manager: Arc<AdapterManager>,
    pub(crate) inactivity_timeout: Duration,
}

impl OffloadTimer {
    pub fn new(manager: Arc<AdapterManager>, inactivity_timeout_secs: u64) -> Self {
        Self {
            manager,
            inactivity_timeout: Duration::from_secs(inactivity_timeout_secs),
        }
    }

    /// Compute the check interval: inactivity_timeout / 10, minimum 60 seconds.
    fn check_interval(&self) -> Duration {
        let interval = self.inactivity_timeout / 10;
        if interval < Duration::from_secs(60) {
            Duration::from_secs(60)
        } else {
            interval
        }
    }

    /// Run the offload timer loop. Intended to be spawned as a background task.
    pub async fn run(&self) {
        let interval = self.check_interval();
        info!(
            "offload timer started: timeout={}s, check_interval={}s",
            self.inactivity_timeout.as_secs(),
            interval.as_secs()
        );

        loop {
            tokio::time::sleep(interval).await;

            let offloaded = self
                .manager
                .offload_inactive_adapters(self.inactivity_timeout)
                .await;

            if !offloaded.is_empty() {
                info!("offloaded {} inactive adapters: {:?}", offloaded.len(), offloaded);
            }
        }
    }
}

#[cfg(test)]
#[path = "offload_test.rs"]
mod tests;
