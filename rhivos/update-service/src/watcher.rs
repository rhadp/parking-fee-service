//! Watcher management for UPDATE_SERVICE.
//!
//! This module manages streaming subscriptions for state updates,
//! allowing clients to receive real-time notifications of adapter state changes.

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

use crate::proto::{AdapterState, AdapterStateEvent};

/// Channel capacity for watcher event buffers.
const WATCHER_CHANNEL_CAPACITY: usize = 100;

/// Watcher entry with sender channel.
struct WatcherEntry {
    sender: mpsc::Sender<Result<AdapterStateEvent, tonic::Status>>,
    id: u64,
}

/// Watcher manager for streaming subscriptions.
pub struct WatcherManager {
    watchers: RwLock<Vec<WatcherEntry>>,
    next_id: RwLock<u64>,
}

impl WatcherManager {
    /// Create a new watcher manager.
    pub fn new() -> Self {
        Self {
            watchers: RwLock::new(Vec::new()),
            next_id: RwLock::new(1),
        }
    }

    /// Register a new watcher and return a receiver for events.
    pub async fn register(
        &self,
    ) -> (
        u64,
        mpsc::Receiver<Result<AdapterStateEvent, tonic::Status>>,
    ) {
        let (sender, receiver) = mpsc::channel(WATCHER_CHANNEL_CAPACITY);

        let mut next_id = self.next_id.write().await;
        let id = *next_id;
        *next_id += 1;

        let entry = WatcherEntry { sender, id };

        let mut watchers = self.watchers.write().await;
        watchers.push(entry);

        debug!(
            "Registered watcher {}, total watchers: {}",
            id,
            watchers.len()
        );

        (id, receiver)
    }

    /// Unregister a watcher by ID.
    pub async fn unregister(&self, watcher_id: u64) {
        let mut watchers = self.watchers.write().await;
        watchers.retain(|w| w.id != watcher_id);
        debug!(
            "Unregistered watcher {}, remaining: {}",
            watcher_id,
            watchers.len()
        );
    }

    /// Broadcast an event to all active watchers.
    pub async fn broadcast(&self, event: AdapterStateEvent) {
        let mut watchers = self.watchers.write().await;
        let mut disconnected = Vec::new();

        for (idx, watcher) in watchers.iter().enumerate() {
            if watcher.sender.send(Ok(event.clone())).await.is_err() {
                disconnected.push(idx);
                debug!("Watcher {} disconnected", watcher.id);
            }
        }

        // Remove disconnected watchers in reverse order to preserve indices
        for idx in disconnected.into_iter().rev() {
            watchers.remove(idx);
        }
    }

    /// Broadcast a state transition event.
    pub async fn broadcast_transition(
        &self,
        adapter_id: &str,
        old_state: AdapterState,
        new_state: AdapterState,
        error_message: Option<&str>,
    ) {
        let event = AdapterStateEvent {
            adapter_id: adapter_id.to_string(),
            old_state: old_state as i32,
            new_state: new_state as i32,
            error_message: error_message.unwrap_or("").to_string(),
        };

        self.broadcast(event).await;
    }

    /// Get the number of active watchers.
    pub async fn watcher_count(&self) -> usize {
        let watchers = self.watchers.read().await;
        watchers.len()
    }

    /// Clean up disconnected watchers.
    pub async fn cleanup_disconnected(&self) {
        let mut watchers = self.watchers.write().await;
        let initial_count = watchers.len();

        watchers.retain(|w| !w.sender.is_closed());

        let removed = initial_count - watchers.len();
        if removed > 0 {
            debug!("Cleaned up {} disconnected watchers", removed);
        }
    }

    /// Send initial state to a specific watcher.
    pub async fn send_initial_state(
        &self,
        watcher_id: u64,
        events: Vec<AdapterStateEvent>,
    ) -> Result<(), ()> {
        let watchers = self.watchers.read().await;
        let watcher = watchers.iter().find(|w| w.id == watcher_id);

        if let Some(watcher) = watcher {
            for event in events {
                if watcher.sender.send(Ok(event)).await.is_err() {
                    warn!("Failed to send initial state to watcher {}", watcher_id);
                    return Err(());
                }
            }
            Ok(())
        } else {
            Err(())
        }
    }
}

impl Default for WatcherManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::Arc;
    use std::time::Duration;

    fn create_test_event(
        adapter_id: &str,
        old: AdapterState,
        new: AdapterState,
    ) -> AdapterStateEvent {
        AdapterStateEvent {
            adapter_id: adapter_id.to_string(),
            old_state: old as i32,
            new_state: new as i32,
            error_message: String::new(),
        }
    }

    #[tokio::test]
    async fn test_register_watcher() {
        let manager = Arc::new(WatcherManager::new());

        let (id1, _rx1) = manager.register().await;
        let (id2, _rx2) = manager.register().await;

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(manager.watcher_count().await, 2);
    }

    #[tokio::test]
    async fn test_unregister_watcher() {
        let manager = Arc::new(WatcherManager::new());

        let (id1, _rx1) = manager.register().await;
        let (id2, _rx2) = manager.register().await;

        assert_eq!(manager.watcher_count().await, 2);

        manager.unregister(id1).await;
        assert_eq!(manager.watcher_count().await, 1);

        manager.unregister(id2).await;
        assert_eq!(manager.watcher_count().await, 0);
    }

    #[tokio::test]
    async fn test_broadcast_event() {
        let manager = Arc::new(WatcherManager::new());

        let (_id, mut rx) = manager.register().await;

        let event = create_test_event(
            "adapter-1",
            AdapterState::Unknown,
            AdapterState::Downloading,
        );
        manager.broadcast(event.clone()).await;

        let received = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("Should receive event")
            .expect("Channel should not be closed");

        let received_event = received.expect("Should be Ok");
        assert_eq!(received_event.adapter_id, "adapter-1");
        assert_eq!(received_event.new_state, AdapterState::Downloading as i32);
    }

    #[tokio::test]
    async fn test_broadcast_transition() {
        let manager = Arc::new(WatcherManager::new());

        let (_id, mut rx) = manager.register().await;

        manager
            .broadcast_transition(
                "adapter-2",
                AdapterState::Downloading,
                AdapterState::Installing,
                None,
            )
            .await;

        let received = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("Should receive event")
            .expect("Channel should not be closed");

        let event = received.expect("Should be Ok");
        assert_eq!(event.adapter_id, "adapter-2");
        assert_eq!(event.old_state, AdapterState::Downloading as i32);
        assert_eq!(event.new_state, AdapterState::Installing as i32);
    }

    #[tokio::test]
    async fn test_cleanup_disconnected() {
        let manager = Arc::new(WatcherManager::new());

        let (_id1, rx1) = manager.register().await;
        let (_id2, _rx2) = manager.register().await;

        assert_eq!(manager.watcher_count().await, 2);

        // Drop rx1 to simulate disconnect
        drop(rx1);

        // Send an event to trigger cleanup
        manager
            .broadcast(create_test_event(
                "test",
                AdapterState::Unknown,
                AdapterState::Running,
            ))
            .await;

        assert_eq!(manager.watcher_count().await, 1);
    }

    #[tokio::test]
    async fn test_send_initial_state() {
        let manager = Arc::new(WatcherManager::new());

        let (id, mut rx) = manager.register().await;

        let events = vec![
            create_test_event("adapter-1", AdapterState::Unknown, AdapterState::Running),
            create_test_event("adapter-2", AdapterState::Unknown, AdapterState::Stopped),
        ];

        let result = manager.send_initial_state(id, events).await;
        assert!(result.is_ok());

        // Should receive both events
        let event1 = rx.recv().await.unwrap().unwrap();
        assert_eq!(event1.adapter_id, "adapter-1");

        let event2 = rx.recv().await.unwrap().unwrap();
        assert_eq!(event2.adapter_id, "adapter-2");
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 11: Watcher Receives State Events
        /// Validates: Requirements 6.1, 6.2, 6.3
        #[test]
        fn prop_watcher_receives_events(
            adapter_id in "[a-z][a-z0-9-]{3,20}",
            new_state in 0i32..=5
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = Arc::new(WatcherManager::new());
                let (_id, mut rx) = manager.register().await;

                let event = AdapterStateEvent {
                    adapter_id: adapter_id.clone(),
                    old_state: AdapterState::Unknown as i32,
                    new_state,
                    error_message: String::new(),
                };

                manager.broadcast(event).await;

                let received = tokio::time::timeout(Duration::from_millis(100), rx.recv())
                    .await
                    .expect("Should receive event")
                    .expect("Channel should not be closed");

                let received_event = received.expect("Should be Ok");
                prop_assert_eq!(received_event.adapter_id, adapter_id);
                prop_assert_eq!(received_event.new_state, new_state);

                Ok(())
            })?;
        }

        /// Property 12: Watcher Cleanup on Disconnect
        /// Validates: Requirements 6.4
        #[test]
        fn prop_watcher_cleanup_preserves_others(
            num_watchers in 2usize..5
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = Arc::new(WatcherManager::new());
                type ReceiverType = mpsc::Receiver<Result<AdapterStateEvent, tonic::Status>>;
                let mut receivers: Vec<ReceiverType> = Vec::new();

                for _ in 0..num_watchers {
                    let (_id, rx) = manager.register().await;
                    receivers.push(rx);
                }

                prop_assert_eq!(manager.watcher_count().await, num_watchers);

                // Drop first receiver to simulate disconnect
                receivers.remove(0);

                // Broadcast to trigger cleanup
                manager.broadcast(create_test_event("test", AdapterState::Unknown, AdapterState::Running)).await;

                prop_assert_eq!(manager.watcher_count().await, num_watchers - 1);

                // Remaining receivers should get events
                for rx in &mut receivers {
                    let result = rx.recv().await;
                    prop_assert!(result.is_some());
                }

                Ok(())
            })?;
        }

        /// Property 13: New Watcher Receives Initial State
        /// Validates: Requirements 6.5
        #[test]
        fn prop_new_watcher_receives_initial_state(
            num_adapters in 1usize..5
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = Arc::new(WatcherManager::new());
                let (id, mut rx): (u64, mpsc::Receiver<Result<AdapterStateEvent, tonic::Status>>) = manager.register().await;

                let events: Vec<AdapterStateEvent> = (0..num_adapters)
                    .map(|i| AdapterStateEvent {
                        adapter_id: format!("adapter-{}", i),
                        old_state: AdapterState::Unknown as i32,
                        new_state: AdapterState::Running as i32,
                        error_message: String::new(),
                    })
                    .collect();

                let result: Result<(), ()> = manager.send_initial_state(id, events).await;
                prop_assert!(result.is_ok());

                // Should receive all initial events
                for i in 0..num_adapters {
                    let received = rx.recv().await;
                    prop_assert!(received.is_some());
                    let event = received.unwrap().unwrap();
                    prop_assert_eq!(event.adapter_id, format!("adapter-{}", i));
                }

                Ok(())
            })?;
        }
    }
}
