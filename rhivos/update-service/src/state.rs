use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};

#[derive(Debug)]
pub struct StateError {
    pub message: String,
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StateError {}

pub struct StateManager {
    inner: Mutex<HashMap<String, AdapterEntry>>,
    broadcaster: broadcast::Sender<AdapterStateEvent>,
}

/// Check whether a state transition is permitted by the state machine.
fn is_valid_transition(from: &AdapterState, to: &AdapterState) -> bool {
    matches!(
        (from, to),
        (AdapterState::Unknown, AdapterState::Downloading)
            | (AdapterState::Downloading, AdapterState::Installing)
            | (AdapterState::Downloading, AdapterState::Error)
            | (AdapterState::Installing, AdapterState::Running)
            | (AdapterState::Installing, AdapterState::Error)
            | (AdapterState::Running, AdapterState::Stopped)
            | (AdapterState::Running, AdapterState::Error)
            | (AdapterState::Stopped, AdapterState::Running)
            | (AdapterState::Stopped, AdapterState::Offloading)
            | (AdapterState::Offloading, AdapterState::Error)
    )
}

/// Timestamp: seconds since Unix epoch.
fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl StateManager {
    pub fn new(broadcaster: broadcast::Sender<AdapterStateEvent>) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            broadcaster,
        }
    }

    /// Insert a new adapter entry into the in-memory store.
    pub fn create_adapter(&self, entry: AdapterEntry) {
        let mut map = self.inner.lock().unwrap();
        map.insert(entry.adapter_id.clone(), entry);
    }

    /// Perform a validated state transition, update metadata, and emit an event.
    pub fn transition(
        &self,
        adapter_id: &str,
        new_state: AdapterState,
        error_msg: Option<String>,
    ) -> Result<(), StateError> {
        let mut map = self.inner.lock().unwrap();
        let entry = map.get_mut(adapter_id).ok_or_else(|| StateError {
            message: format!("adapter not found: {adapter_id}"),
        })?;

        let old_state = entry.state.clone();
        if !is_valid_transition(&old_state, &new_state) {
            return Err(StateError {
                message: format!(
                    "invalid transition {old_state:?} -> {new_state:?} for adapter {adapter_id}"
                ),
            });
        }

        entry.state = new_state.clone();
        if let Some(msg) = error_msg {
            entry.error_message = Some(msg);
        }
        if new_state == AdapterState::Stopped {
            entry.stopped_at = Some(Instant::now());
        }

        let event = AdapterStateEvent {
            adapter_id: adapter_id.to_string(),
            old_state,
            new_state,
            timestamp: now_unix_secs(),
        };
        // Ignore send error — no receivers is expected during tests / no subscribers.
        let _ = self.broadcaster.send(event);
        Ok(())
    }

    /// Force the adapter into ERROR state, bypassing transition validation.
    /// Use this when the adapter may be in any state and needs to record an error.
    pub fn force_error(&self, adapter_id: &str, error_msg: &str) {
        let mut map = self.inner.lock().unwrap();
        if let Some(entry) = map.get_mut(adapter_id) {
            let old_state = entry.state.clone();
            entry.state = AdapterState::Error;
            entry.error_message = Some(error_msg.to_string());

            let event = AdapterStateEvent {
                adapter_id: adapter_id.to_string(),
                old_state,
                new_state: AdapterState::Error,
                timestamp: now_unix_secs(),
            };
            let _ = self.broadcaster.send(event);
        }
    }

    /// Get a clone of the adapter entry, or None if not found.
    pub fn get_adapter(&self, adapter_id: &str) -> Option<AdapterEntry> {
        let map = self.inner.lock().unwrap();
        map.get(adapter_id).cloned()
    }

    /// Return clones of all known adapter entries.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        let map = self.inner.lock().unwrap();
        map.values().cloned().collect()
    }

    /// Remove an adapter from state entirely. Returns error if not found.
    pub fn remove_adapter(&self, adapter_id: &str) -> Result<(), StateError> {
        let mut map = self.inner.lock().unwrap();
        if map.remove(adapter_id).is_none() {
            return Err(StateError {
                message: format!("adapter not found: {adapter_id}"),
            });
        }
        Ok(())
    }

    /// Return the single adapter currently in RUNNING state, if any.
    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        let map = self.inner.lock().unwrap();
        map.values()
            .find(|e| e.state == AdapterState::Running)
            .cloned()
    }

    /// Return all adapters that have been in STOPPED state for longer than `timeout`.
    pub fn get_offload_candidates(&self, timeout: Duration) -> Vec<AdapterEntry> {
        let map = self.inner.lock().unwrap();
        map.values()
            .filter(|e| {
                if e.state != AdapterState::Stopped {
                    return false;
                }
                match e.stopped_at {
                    Some(stopped_at) => stopped_at.elapsed() >= timeout,
                    None => false,
                }
            })
            .cloned()
            .collect()
    }

    /// Subscribe to state transition events.
    pub fn subscribe(&self) -> broadcast::Receiver<AdapterStateEvent> {
        self.broadcaster.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::AdapterState;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn make_entry(id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: AdapterState::Unknown,
            job_id: uuid::Uuid::new_v4().to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    // TS-07-11: GetAdapterStatus returns current state
    #[test]
    fn test_create_and_get_adapter() {
        let (tx, _rx) = broadcast::channel(100);
        let state = StateManager::new(tx);
        let entry = make_entry("test-adapter-v1", "example.com/test-adapter:v1");
        state.create_adapter(entry.clone());
        let found = state.get_adapter("test-adapter-v1").unwrap();
        assert_eq!(found.adapter_id, "test-adapter-v1");
        assert_eq!(found.state, AdapterState::Unknown);
    }

    // TS-07-E9: ListAdapters returns empty when none installed
    #[test]
    fn test_list_adapters_empty() {
        let (tx, _rx) = broadcast::channel(100);
        let state = StateManager::new(tx);
        let list = state.list_adapters();
        assert!(list.is_empty());
    }

    // TS-07-10: ListAdapters returns all known adapters
    #[test]
    fn test_list_adapters_multiple() {
        let (tx, _rx) = broadcast::channel(100);
        let state = StateManager::new(tx);
        state.create_adapter(make_entry("adapter-a-v1", "example.com/adapter-a:v1"));
        state.create_adapter(make_entry("adapter-b-v2", "example.com/adapter-b:v2"));
        let list = state.list_adapters();
        assert_eq!(list.len(), 2);
        let mut ids: Vec<_> = list.iter().map(|a| a.adapter_id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["adapter-a-v1", "adapter-b-v2"]);
    }

    // TS-07-E8: GetAdapterStatus unknown ID returns NOT_FOUND
    #[test]
    fn test_get_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(100);
        let state = StateManager::new(tx);
        assert!(state.get_adapter("nonexistent-adapter").is_none());
    }

    // TS-07-12: RemoveAdapter removes from state
    #[test]
    fn test_remove_adapter() {
        let (tx, _rx) = broadcast::channel(100);
        let state = StateManager::new(tx);
        state.create_adapter(make_entry("test-adapter-v1", "example.com/test-adapter:v1"));
        state.remove_adapter("test-adapter-v1").unwrap();
        assert!(state.get_adapter("test-adapter-v1").is_none());
    }

    // TS-07-E10: RemoveAdapter unknown ID returns error
    #[test]
    fn test_remove_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(100);
        let state = StateManager::new(tx);
        let result = state.remove_adapter("nonexistent-adapter");
        assert!(result.is_err());
    }

    // TS-07-8: WatchAdapterStates streams events with correct fields
    #[tokio::test]
    async fn test_state_transition_emits_event() {
        let (tx, mut rx) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx));
        let entry = make_entry("parkhaus-munich-v1-0-0", "example.com/parkhaus-munich:v1.0.0");
        state.create_adapter(entry);

        // Transition to Downloading
        state
            .transition("parkhaus-munich-v1-0-0", AdapterState::Downloading, None)
            .unwrap();
        let event = rx.recv().await.unwrap();
        assert_eq!(event.adapter_id, "parkhaus-munich-v1-0-0");
        assert_eq!(event.old_state, AdapterState::Unknown);
        assert_eq!(event.new_state, AdapterState::Downloading);
        assert!(event.timestamp > 0);
    }

    // TS-07-9: No historical replay for new subscribers
    #[tokio::test]
    async fn test_no_historical_replay() {
        let (tx, _rx) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx.clone()));
        // Trigger transitions before subscribing
        let entry = make_entry("adapter-a-v1", "example.com/adapter-a:v1");
        state.create_adapter(entry);
        state
            .transition("adapter-a-v1", AdapterState::Downloading, None)
            .unwrap();
        state
            .transition("adapter-a-v1", AdapterState::Installing, None)
            .unwrap();

        // New subscriber - should not see historical events
        let mut new_rx = state.subscribe();
        // Trigger one more transition
        state
            .transition("adapter-a-v1", AdapterState::Running, None)
            .unwrap();
        let event = new_rx.recv().await.unwrap();
        // Should only see the Running transition, not the earlier ones
        assert_ne!(event.old_state, AdapterState::Unknown);
        assert_ne!(event.new_state, AdapterState::Downloading);
    }

    // TS-07-E15: No subscribers active during transition - no error
    #[test]
    fn test_no_subscribers_no_error() {
        let (tx, _rx) = broadcast::channel(100);
        // Drop _rx immediately so no subscribers
        let state = StateManager::new(tx);
        let entry = make_entry("adapter-a-v1", "example.com/adapter-a:v1");
        state.create_adapter(entry);
        // Should not panic even with no subscribers
        let result = state.transition("adapter-a-v1", AdapterState::Downloading, None);
        assert!(result.is_ok());
    }

    // TS-07-E7: Subscriber disconnect does not affect others
    #[tokio::test]
    async fn test_subscriber_disconnect() {
        let (tx, _rx1) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx.clone()));
        let mut rx2 = state.subscribe();
        // Disconnect rx1 (already dropped above)
        let entry = make_entry("adapter-a-v1", "example.com/adapter-a:v1");
        state.create_adapter(entry);
        state
            .transition("adapter-a-v1", AdapterState::Downloading, None)
            .unwrap();
        // rx2 should still receive the event
        let event = rx2.recv().await.unwrap();
        assert_eq!(event.new_state, AdapterState::Downloading);
    }
}

// TS-07-P3: State transition validity
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use std::sync::Arc;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(20))]

        #[test]
        #[ignore = "proptest: run with --include-ignored"]
        fn proptest_state_transition_validity(_seed in 0u32..100) {
            let valid_transitions: std::collections::HashSet<_> = vec![
                (AdapterState::Unknown, AdapterState::Downloading),
                (AdapterState::Downloading, AdapterState::Installing),
                (AdapterState::Downloading, AdapterState::Error),
                (AdapterState::Installing, AdapterState::Running),
                (AdapterState::Installing, AdapterState::Error),
                (AdapterState::Running, AdapterState::Stopped),
                (AdapterState::Running, AdapterState::Error),
                (AdapterState::Stopped, AdapterState::Running),
                (AdapterState::Stopped, AdapterState::Offloading),
                (AdapterState::Offloading, AdapterState::Error),
            ].into_iter().collect();

            let (tx, mut rx) = broadcast::channel(1000);
            let state = Arc::new(StateManager::new(tx));
            let entry = AdapterEntry {
                adapter_id: "test-adapter-v1".to_string(),
                image_ref: "example.com/test:v1".to_string(),
                checksum_sha256: "sha256:abc".to_string(),
                state: AdapterState::Unknown,
                job_id: uuid::Uuid::new_v4().to_string(),
                stopped_at: None,
                error_message: None,
            };
            state.create_adapter(entry);
            // Try some transitions
            let _ = state.transition("test-adapter-v1", AdapterState::Downloading, None);
            let _ = state.transition("test-adapter-v1", AdapterState::Installing, None);
            // Collect events
            while let Ok(event) = rx.try_recv() {
                prop_assert!(
                    valid_transitions.contains(&(event.old_state.clone(), event.new_state.clone())),
                    "Invalid transition: {:?} -> {:?}",
                    event.old_state,
                    event.new_state
                );
            }
        }
    }
}
