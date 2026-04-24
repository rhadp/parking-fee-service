use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::broadcast;

use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};

/// Error type for state manager operations.
#[derive(Debug)]
pub enum StateError {
    NotFound(String),
    Internal(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::NotFound(msg) => write!(f, "not found: {msg}"),
            StateError::Internal(msg) => write!(f, "internal: {msg}"),
        }
    }
}

impl std::error::Error for StateError {}

/// Thread-safe in-memory state manager for adapter entries.
///
/// Emits `AdapterStateEvent` messages via a broadcast channel on every
/// state transition.
#[allow(dead_code)]
pub struct StateManager {
    adapters: Arc<Mutex<HashMap<String, AdapterEntry>>>,
    broadcaster: broadcast::Sender<AdapterStateEvent>,
}

impl StateManager {
    /// Creates a new state manager wired to the given broadcast sender.
    pub fn new(broadcaster: broadcast::Sender<AdapterStateEvent>) -> Self {
        Self {
            adapters: Arc::new(Mutex::new(HashMap::new())),
            broadcaster,
        }
    }

    /// Inserts a new adapter entry.
    pub fn create_adapter(&self, _entry: AdapterEntry) {
        todo!("create_adapter not yet implemented")
    }

    /// Transitions the adapter to a new state and emits an event.
    pub fn transition(
        &self,
        _adapter_id: &str,
        _new_state: AdapterState,
        _error_msg: Option<String>,
    ) -> Result<(), StateError> {
        todo!("transition not yet implemented")
    }

    /// Returns a clone of the adapter entry if found.
    pub fn get_adapter(&self, _adapter_id: &str) -> Option<AdapterEntry> {
        todo!("get_adapter not yet implemented")
    }

    /// Returns a list of all known adapter entries.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        todo!("list_adapters not yet implemented")
    }

    /// Removes an adapter from in-memory state.
    pub fn remove_adapter(&self, _adapter_id: &str) -> Result<(), StateError> {
        todo!("remove_adapter not yet implemented")
    }

    /// Returns the adapter currently in RUNNING state, if any.
    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        todo!("get_running_adapter not yet implemented")
    }

    /// Returns adapters that have been STOPPED longer than `timeout`.
    pub fn get_offload_candidates(&self, _timeout: Duration) -> Vec<AdapterEntry> {
        todo!("get_offload_candidates not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Helper: create a minimal adapter entry for testing.
    fn test_entry(adapter_id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:test".to_string(),
            state: AdapterState::Unknown,
            job_id: "test-job-id".to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    // TS-07-11: GetAdapterStatus returns current state
    #[test]
    fn test_create_and_get_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let entry = test_entry("parkhaus-munich-v1.0.0", "example.com/parkhaus-munich:v1.0.0");
        mgr.create_adapter(entry);
        let result = mgr.get_adapter("parkhaus-munich-v1.0.0");
        assert!(result.is_some());
        let adapter = result.unwrap();
        assert_eq!(adapter.adapter_id, "parkhaus-munich-v1.0.0");
        assert_eq!(adapter.state, AdapterState::Unknown);
    }

    // TS-07-E9: ListAdapters returns empty when none installed
    #[test]
    fn test_list_adapters_empty() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let adapters = mgr.list_adapters();
        assert!(adapters.is_empty());
    }

    // TS-07-10: ListAdapters returns all known adapters
    #[test]
    fn test_list_adapters_multiple() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let entry_a = test_entry("adapter-a-v1", "example.com/adapter-a:v1");
        let entry_b = test_entry("adapter-b-v2", "example.com/adapter-b:v2");
        mgr.create_adapter(entry_a);
        mgr.create_adapter(entry_b);
        let adapters = mgr.list_adapters();
        assert_eq!(adapters.len(), 2);
        let mut ids: Vec<String> = adapters.iter().map(|a| a.adapter_id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["adapter-a-v1", "adapter-b-v2"]);
    }

    // TS-07-E8: GetAdapterStatus with unknown ID returns None
    #[test]
    fn test_get_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let result = mgr.get_adapter("nonexistent-adapter");
        assert!(result.is_none());
    }

    // TS-07-12: RemoveAdapter removes from state
    #[test]
    fn test_remove_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let entry = test_entry("adapter-v1", "example.com/adapter:v1");
        mgr.create_adapter(entry);
        let result = mgr.remove_adapter("adapter-v1");
        assert!(result.is_ok());
        assert!(mgr.get_adapter("adapter-v1").is_none());
    }

    // TS-07-E10: RemoveAdapter with unknown ID returns error
    #[test]
    fn test_remove_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let result = mgr.remove_adapter("nonexistent-adapter");
        assert!(result.is_err());
    }

    // TS-07-8: State transitions emit events with correct fields
    #[tokio::test]
    async fn test_state_transition_emits_event() {
        let (tx, mut rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let entry = test_entry("adapter-v1", "example.com/adapter:v1");
        mgr.create_adapter(entry);

        mgr.transition("adapter-v1", AdapterState::Downloading, None)
            .unwrap();

        let event = rx.try_recv().unwrap();
        assert_eq!(event.adapter_id, "adapter-v1");
        assert_eq!(event.old_state, AdapterState::Unknown);
        assert_eq!(event.new_state, AdapterState::Downloading);
        assert!(event.timestamp > 0);
    }

    // TS-07-9: New subscribers do not receive historical events
    #[tokio::test]
    async fn test_no_historical_replay() {
        let (tx, _rx1) = broadcast::channel(16);
        let mgr = StateManager::new(tx.clone());
        let entry = test_entry("adapter-v1", "example.com/adapter:v1");
        mgr.create_adapter(entry);

        // Transition before the new subscriber connects
        mgr.transition("adapter-v1", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Running, None)
            .unwrap();

        // New subscriber connects after transitions
        let mut rx2 = tx.subscribe();

        // Trigger a new transition
        mgr.transition("adapter-v1", AdapterState::Stopped, None)
            .unwrap();

        let event = rx2.try_recv().unwrap();
        // Should only get the RUNNING->STOPPED event, not historical ones
        assert_eq!(event.old_state, AdapterState::Running);
        assert_eq!(event.new_state, AdapterState::Stopped);
        // No more events
        assert!(rx2.try_recv().is_err());
    }

    // TS-07-E15: No subscribers active during transition - no errors
    #[test]
    fn test_no_subscribers_no_error() {
        let (tx, rx) = broadcast::channel::<AdapterStateEvent>(16);
        // Drop all receivers so there are no active subscribers
        drop(rx);
        let mgr = StateManager::new(tx);
        let entry = test_entry("adapter-v1", "example.com/adapter:v1");
        mgr.create_adapter(entry);

        // This should not panic even with no subscribers
        let result = mgr.transition("adapter-v1", AdapterState::Downloading, None);
        assert!(result.is_ok());

        // State should still be updated
        let adapter = mgr.get_adapter("adapter-v1").unwrap();
        assert_eq!(adapter.state, AdapterState::Downloading);
    }

    // TS-07-E7: Subscriber disconnect does not affect others
    #[tokio::test]
    async fn test_subscriber_disconnect() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx.clone());
        let entry = test_entry("adapter-v1", "example.com/adapter:v1");
        mgr.create_adapter(entry);

        let rx1 = tx.subscribe();
        let mut rx2 = tx.subscribe();

        // Drop subscriber 1
        drop(rx1);

        // Trigger a transition
        mgr.transition("adapter-v1", AdapterState::Downloading, None)
            .unwrap();

        // Subscriber 2 should still receive the event
        let event = rx2.try_recv().unwrap();
        assert_eq!(event.new_state, AdapterState::Downloading);
    }
}
