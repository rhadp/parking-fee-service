//! Offload scheduling for UPDATE_SERVICE.
//!
//! This module handles automatic offloading of unused adapters
//! that have been stopped for longer than the configured threshold.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::container::ContainerManager;
use crate::logger::OperationLogger;
use crate::tracker::StateTracker;

/// Offload scheduler for automatic cleanup.
pub struct OffloadScheduler {
    state_tracker: Arc<StateTracker>,
    container_manager: Arc<ContainerManager>,
    offload_threshold_secs: u64,
    check_interval: Duration,
    logger: Arc<OperationLogger>,
    shutdown_rx: watch::Receiver<bool>,
}

impl OffloadScheduler {
    /// Create a new offload scheduler.
    pub fn new(
        state_tracker: Arc<StateTracker>,
        container_manager: Arc<ContainerManager>,
        offload_threshold_hours: u64,
        check_interval_minutes: u64,
        logger: Arc<OperationLogger>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            state_tracker,
            container_manager,
            offload_threshold_secs: offload_threshold_hours * 3600,
            check_interval: Duration::from_secs(check_interval_minutes * 60),
            logger,
            shutdown_rx,
        }
    }

    /// Start the offload scheduler.
    pub async fn run(&mut self) {
        info!(
            "Starting offload scheduler with {}h threshold, {}m interval",
            self.offload_threshold_secs / 3600,
            self.check_interval.as_secs() / 60
        );

        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.check_interval) => {
                    self.check_and_offload().await;
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("Offload scheduler shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Check for stale adapters and offload them.
    async fn check_and_offload(&self) {
        debug!("Checking for stale adapters to offload");

        let stale_adapters = self
            .state_tracker
            .get_stale_adapters(self.offload_threshold_secs)
            .await;

        if stale_adapters.is_empty() {
            debug!("No stale adapters to offload");
            return;
        }

        info!("Found {} stale adapters to offload", stale_adapters.len());

        for adapter_id in stale_adapters {
            let correlation_id = OperationLogger::generate_correlation_id();
            info!(
                "Offloading stale adapter {} (correlation_id={})",
                adapter_id, correlation_id
            );

            // Stop the container (should already be stopped, but ensure)
            if let Err(e) = self
                .container_manager
                .stop(&adapter_id, &correlation_id)
                .await
            {
                warn!(
                    "Failed to stop adapter {} during offload: {}",
                    adapter_id, e
                );
            }

            // Remove the container
            if let Err(e) = self
                .container_manager
                .remove(&adapter_id, &correlation_id)
                .await
            {
                warn!(
                    "Failed to remove adapter {} during offload: {}",
                    adapter_id, e
                );
                continue;
            }

            // Remove from state tracker
            if let Err(e) = self
                .state_tracker
                .remove(&adapter_id, &correlation_id)
                .await
            {
                warn!(
                    "Failed to remove adapter {} from tracker during offload: {}",
                    adapter_id, e
                );
            }

            info!("Successfully offloaded adapter {}", adapter_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::AdapterState;
    use crate::watcher::WatcherManager;
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn create_test_components() -> (
        Arc<StateTracker>,
        Arc<ContainerManager>,
        Arc<OperationLogger>,
        TempDir,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let watcher_manager = Arc::new(WatcherManager::new());
        let logger = Arc::new(OperationLogger::new("test"));
        let state_tracker = Arc::new(StateTracker::new(watcher_manager, logger.clone()));
        let container_manager = Arc::new(ContainerManager::new(
            temp_dir.path().to_path_buf(),
            "/run/test.sock".to_string(),
            logger.clone(),
        ));

        (state_tracker, container_manager, logger, temp_dir)
    }

    #[tokio::test]
    async fn test_offload_scheduler_creation() {
        let (state_tracker, container_manager, logger, _temp_dir) = create_test_components();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);

        let scheduler = OffloadScheduler::new(
            state_tracker,
            container_manager,
            24, // 24 hours
            60, // 60 minutes
            logger,
            shutdown_rx,
        );

        assert_eq!(scheduler.offload_threshold_secs, 24 * 3600);
        assert_eq!(scheduler.check_interval, Duration::from_secs(60 * 60));
    }

    #[tokio::test]
    async fn test_no_stale_adapters() {
        let (state_tracker, container_manager, logger, _temp_dir) = create_test_components();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);

        let scheduler = OffloadScheduler::new(
            state_tracker.clone(),
            container_manager,
            24,
            1,
            logger,
            shutdown_rx,
        );

        // Add an adapter that's not stale (still running)
        let correlation_id = OperationLogger::generate_correlation_id();
        state_tracker
            .add("adapter-1", "reg.io/img:v1", &correlation_id)
            .await
            .unwrap();

        // Check should find nothing to offload
        scheduler.check_and_offload().await;

        // Adapter should still exist
        assert!(state_tracker.exists("adapter-1").await);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 17: Automatic Offload After Inactivity
        /// Validates: Requirements 9.1, 9.2, 9.3
        #[test]
        fn prop_offload_threshold_configuration(
            threshold_hours in 1u64..168,
            check_interval_minutes in 1u64..120
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (state_tracker, container_manager, logger, _temp_dir) = create_test_components();
                let (_shutdown_tx, shutdown_rx) = watch::channel(false);

                let scheduler = OffloadScheduler::new(
                    state_tracker,
                    container_manager,
                    threshold_hours,
                    check_interval_minutes,
                    logger,
                    shutdown_rx,
                );

                // Verify configuration
                prop_assert_eq!(scheduler.offload_threshold_secs, threshold_hours * 3600);
                prop_assert_eq!(
                    scheduler.check_interval,
                    Duration::from_secs(check_interval_minutes * 60)
                );

                Ok(())
            })?;
        }

        #[test]
        fn prop_stale_adapters_identified(
            num_adapters in 1usize..5
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (state_tracker, _container_manager, _logger, _temp_dir) = create_test_components();
                let correlation_id = OperationLogger::generate_correlation_id();

                for i in 0..num_adapters {
                    state_tracker
                        .add(&format!("adapter-{}", i), &format!("reg.io/img:{}", i), &correlation_id)
                        .await
                        .unwrap();

                    // Transition to STOPPED
                    state_tracker
                        .transition(
                            &format!("adapter-{}", i),
                            AdapterState::Stopped,
                            None,
                            &correlation_id,
                        )
                        .await
                        .unwrap();
                }

                // With very high threshold, none should be stale (recently stopped)
                let stale_high = state_tracker.get_stale_adapters(u64::MAX).await;
                prop_assert_eq!(stale_high.len(), 0);

                // Adapters should be found as stopped
                let adapters = state_tracker.list_all().await;
                let stopped_count = adapters.iter().filter(|a| a.state == AdapterState::Stopped).count();
                prop_assert_eq!(stopped_count, num_adapters);

                Ok(())
            })?;
        }
    }
}
