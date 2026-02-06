//! State tracking for UPDATE_SERVICE.
//!
//! This module tracks adapter states and manages state transitions,
//! notifying watchers of changes.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::debug;

use crate::container::ContainerManager;
use crate::error::UpdateError;
use crate::logger::OperationLogger;
use crate::proto::AdapterState;
use crate::state::AdapterEntry;
use crate::watcher::WatcherManager;

/// State tracker for adapters.
pub struct StateTracker {
    adapters: RwLock<HashMap<String, AdapterEntry>>,
    watcher_manager: Arc<WatcherManager>,
    logger: Arc<OperationLogger>,
}

impl StateTracker {
    /// Create a new state tracker.
    pub fn new(watcher_manager: Arc<WatcherManager>, logger: Arc<OperationLogger>) -> Self {
        Self {
            adapters: RwLock::new(HashMap::new()),
            watcher_manager,
            logger,
        }
    }

    /// Get the current state of an adapter.
    pub async fn get_state(&self, adapter_id: &str) -> Option<AdapterState> {
        let adapters = self.adapters.read().await;
        adapters.get(adapter_id).map(|e| e.state)
    }

    /// Get the full entry for an adapter.
    pub async fn get_entry(&self, adapter_id: &str) -> Option<AdapterEntry> {
        let adapters = self.adapters.read().await;
        adapters.get(adapter_id).cloned()
    }

    /// Check if an adapter exists.
    pub async fn exists(&self, adapter_id: &str) -> bool {
        let adapters = self.adapters.read().await;
        adapters.contains_key(adapter_id)
    }

    /// Add a new adapter with initial DOWNLOADING state.
    pub async fn add(
        &self,
        adapter_id: &str,
        image_ref: &str,
        correlation_id: &str,
    ) -> Result<(), UpdateError> {
        let mut adapters = self.adapters.write().await;

        if adapters.contains_key(adapter_id) {
            return Err(UpdateError::AdapterAlreadyExists(adapter_id.to_string()));
        }

        let entry = AdapterEntry::new(adapter_id.to_string(), image_ref.to_string());
        adapters.insert(adapter_id.to_string(), entry);

        debug!("Added adapter {} with DOWNLOADING state", adapter_id);

        // Log state transition
        self.logger.log_state_transition(
            correlation_id,
            adapter_id,
            AdapterState::Unknown,
            AdapterState::Downloading,
            Some("Initial state"),
        );

        // Notify watchers
        self.watcher_manager
            .broadcast_transition(
                adapter_id,
                AdapterState::Unknown,
                AdapterState::Downloading,
                None,
            )
            .await;

        Ok(())
    }

    /// Transition an adapter to a new state.
    pub async fn transition(
        &self,
        adapter_id: &str,
        new_state: AdapterState,
        error_message: Option<String>,
        correlation_id: &str,
    ) -> Result<(), UpdateError> {
        let mut adapters = self.adapters.write().await;

        let entry = adapters
            .get_mut(adapter_id)
            .ok_or_else(|| UpdateError::AdapterNotFound(adapter_id.to_string()))?;

        let old_state = entry.state;
        entry.set_state(new_state, error_message.clone());

        debug!(
            "Adapter {} transitioned from {:?} to {:?}",
            adapter_id, old_state, new_state
        );

        // Log state transition
        self.logger.log_state_transition(
            correlation_id,
            adapter_id,
            old_state,
            new_state,
            error_message.as_deref(),
        );

        // Notify watchers
        self.watcher_manager
            .broadcast_transition(adapter_id, old_state, new_state, error_message.as_deref())
            .await;

        Ok(())
    }

    /// Update the last activity time for an adapter.
    pub async fn touch(&self, adapter_id: &str) -> Result<(), UpdateError> {
        let mut adapters = self.adapters.write().await;

        let entry = adapters
            .get_mut(adapter_id)
            .ok_or_else(|| UpdateError::AdapterNotFound(adapter_id.to_string()))?;

        entry.touch();
        Ok(())
    }

    /// Remove an adapter from tracking.
    pub async fn remove(&self, adapter_id: &str, correlation_id: &str) -> Result<(), UpdateError> {
        let mut adapters = self.adapters.write().await;

        let entry = adapters
            .remove(adapter_id)
            .ok_or_else(|| UpdateError::AdapterNotFound(adapter_id.to_string()))?;

        debug!("Removed adapter {} from tracking", adapter_id);

        // Log state transition
        self.logger.log_state_transition(
            correlation_id,
            adapter_id,
            entry.state,
            AdapterState::Unknown,
            Some("Uninstalled"),
        );

        // Notify watchers about removal
        self.watcher_manager
            .broadcast_transition(
                adapter_id,
                entry.state,
                AdapterState::Unknown,
                Some("Removed"),
            )
            .await;

        Ok(())
    }

    /// List all adapters.
    pub async fn list_all(&self) -> Vec<AdapterEntry> {
        let adapters = self.adapters.read().await;
        adapters.values().cloned().collect()
    }

    /// Get initial state events for all adapters.
    pub async fn get_initial_events(&self) -> Vec<crate::proto::AdapterStateEvent> {
        let adapters = self.adapters.read().await;
        adapters
            .values()
            .map(|entry| crate::proto::AdapterStateEvent {
                adapter_id: entry.adapter_id.clone(),
                old_state: AdapterState::Unknown as i32,
                new_state: entry.state as i32,
                error_message: entry.error_message.clone().unwrap_or_default(),
            })
            .collect()
    }

    /// Restore state from running containers on startup.
    pub async fn restore_from_containers(
        &self,
        container_manager: &ContainerManager,
    ) -> Result<(), UpdateError> {
        let running = container_manager.list_running().await?;
        let mut adapters = self.adapters.write().await;

        for container_name in running {
            // Assume container name is the adapter ID
            let entry = AdapterEntry {
                adapter_id: container_name.clone(),
                image_ref: String::new(), // Unknown after restart
                state: AdapterState::Running,
                error_message: None,
                last_updated: std::time::SystemTime::now(),
                last_activity: std::time::SystemTime::now(),
            };

            adapters.insert(container_name.clone(), entry);
            debug!("Restored adapter {} as RUNNING", container_name);
        }

        Ok(())
    }

    /// Get adapters that have been stopped for longer than the threshold.
    pub async fn get_stale_adapters(&self, threshold_secs: u64) -> Vec<String> {
        let adapters = self.adapters.read().await;
        adapters
            .values()
            .filter(|e| e.state == AdapterState::Stopped && e.inactivity_seconds() > threshold_secs)
            .map(|e| e.adapter_id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::time::Duration;

    fn create_test_tracker() -> StateTracker {
        let watcher_manager = Arc::new(WatcherManager::new());
        let logger = Arc::new(OperationLogger::new("test"));
        StateTracker::new(watcher_manager, logger)
    }

    #[tokio::test]
    async fn test_add_adapter() {
        let tracker = create_test_tracker();
        let correlation_id = OperationLogger::generate_correlation_id();

        let result = tracker
            .add("adapter-1", "registry.io/image:v1", &correlation_id)
            .await;

        assert!(result.is_ok());
        assert!(tracker.exists("adapter-1").await);
        assert_eq!(
            tracker.get_state("adapter-1").await,
            Some(AdapterState::Downloading)
        );
    }

    #[tokio::test]
    async fn test_add_duplicate_adapter() {
        let tracker = create_test_tracker();
        let correlation_id = OperationLogger::generate_correlation_id();

        tracker
            .add("adapter-1", "registry.io/image:v1", &correlation_id)
            .await
            .unwrap();

        let result = tracker
            .add("adapter-1", "registry.io/image:v2", &correlation_id)
            .await;

        assert!(matches!(result, Err(UpdateError::AdapterAlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_transition_state() {
        let tracker = create_test_tracker();
        let correlation_id = OperationLogger::generate_correlation_id();

        tracker
            .add("adapter-1", "registry.io/image:v1", &correlation_id)
            .await
            .unwrap();

        tracker
            .transition("adapter-1", AdapterState::Installing, None, &correlation_id)
            .await
            .unwrap();

        assert_eq!(
            tracker.get_state("adapter-1").await,
            Some(AdapterState::Installing)
        );

        tracker
            .transition("adapter-1", AdapterState::Running, None, &correlation_id)
            .await
            .unwrap();

        assert_eq!(
            tracker.get_state("adapter-1").await,
            Some(AdapterState::Running)
        );
    }

    #[tokio::test]
    async fn test_transition_nonexistent() {
        let tracker = create_test_tracker();
        let correlation_id = OperationLogger::generate_correlation_id();

        let result = tracker
            .transition("nonexistent", AdapterState::Running, None, &correlation_id)
            .await;

        assert!(matches!(result, Err(UpdateError::AdapterNotFound(_))));
    }

    #[tokio::test]
    async fn test_remove_adapter() {
        let tracker = create_test_tracker();
        let correlation_id = OperationLogger::generate_correlation_id();

        tracker
            .add("adapter-1", "registry.io/image:v1", &correlation_id)
            .await
            .unwrap();

        assert!(tracker.exists("adapter-1").await);

        tracker.remove("adapter-1", &correlation_id).await.unwrap();

        assert!(!tracker.exists("adapter-1").await);
    }

    #[tokio::test]
    async fn test_list_all() {
        let tracker = create_test_tracker();
        let correlation_id = OperationLogger::generate_correlation_id();

        tracker
            .add("adapter-1", "registry.io/image:v1", &correlation_id)
            .await
            .unwrap();
        tracker
            .add("adapter-2", "registry.io/image:v2", &correlation_id)
            .await
            .unwrap();

        let adapters = tracker.list_all().await;
        assert_eq!(adapters.len(), 2);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 10: State Timestamp Updates
        /// Validates: Requirements 5.2, 5.3
        #[test]
        fn prop_state_timestamp_updated(
            adapter_id in "[a-z][a-z0-9-]{3,20}",
            image_ref in "registry\\.[a-z]+\\.[a-z]+/[a-z]+:[a-z0-9]+"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let tracker = create_test_tracker();
                let correlation_id = OperationLogger::generate_correlation_id();

                tracker.add(&adapter_id, &image_ref, &correlation_id).await.unwrap();

                let entry_before = tracker.get_entry(&adapter_id).await.unwrap();
                let time_before = entry_before.last_updated;

                // Small delay
                tokio::time::sleep(Duration::from_millis(1)).await;

                tracker
                    .transition(&adapter_id, AdapterState::Running, None, &correlation_id)
                    .await
                    .unwrap();

                let entry_after = tracker.get_entry(&adapter_id).await.unwrap();

                prop_assert!(entry_after.last_updated >= time_before);
                prop_assert_eq!(entry_after.state, AdapterState::Running);
                prop_assert_eq!(entry_after.adapter_id, adapter_id);

                Ok(())
            })?;
        }

        /// Property 14: List Adapters Returns Complete Info
        /// Validates: Requirements 7.1, 7.2
        #[test]
        fn prop_list_adapters_complete(
            num_adapters in 1usize..5
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let tracker = create_test_tracker();
                let correlation_id = OperationLogger::generate_correlation_id();

                for i in 0..num_adapters {
                    tracker
                        .add(&format!("adapter-{}", i), &format!("reg.io/img:{}", i), &correlation_id)
                        .await
                        .unwrap();
                }

                let adapters = tracker.list_all().await;
                prop_assert_eq!(adapters.len(), num_adapters);

                for adapter in &adapters {
                    prop_assert!(!adapter.adapter_id.is_empty());
                    prop_assert!(!adapter.image_ref.is_empty());
                    // Should have required fields
                    let _state = adapter.state; // Should be valid
                    let _last_updated = adapter.last_updated; // Should be set
                }

                Ok(())
            })?;
        }
    }
}
