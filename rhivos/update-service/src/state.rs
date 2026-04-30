use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;

use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};

/// Error type for state operations.
#[derive(Debug)]
pub enum StateError {
    NotFound(String),
    InvalidTransition(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::NotFound(msg) => write!(f, "not found: {msg}"),
            StateError::InvalidTransition(msg) => write!(f, "invalid transition: {msg}"),
        }
    }
}

impl std::error::Error for StateError {}

/// Thread-safe in-memory adapter state manager.
pub struct StateManager {
    #[allow(dead_code)]
    adapters: Arc<Mutex<HashMap<String, AdapterEntry>>>,
    #[allow(dead_code)]
    broadcaster: broadcast::Sender<AdapterStateEvent>,
}

impl StateManager {
    /// Creates a new state manager with the given broadcast channel.
    pub fn new(_broadcaster: broadcast::Sender<AdapterStateEvent>) -> Self {
        todo!("StateManager::new not yet implemented")
    }

    /// Inserts a new adapter entry.
    pub fn create_adapter(&self, _entry: AdapterEntry) {
        todo!("create_adapter not yet implemented")
    }

    /// Transitions an adapter to a new state, emitting an event.
    pub fn transition(
        &self,
        _adapter_id: &str,
        _new_state: AdapterState,
        _error_msg: Option<String>,
    ) -> Result<(), StateError> {
        todo!("transition not yet implemented")
    }

    /// Returns a clone of the adapter entry if it exists.
    pub fn get_adapter(&self, _adapter_id: &str) -> Option<AdapterEntry> {
        todo!("get_adapter not yet implemented")
    }

    /// Returns a list of all known adapters.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        todo!("list_adapters not yet implemented")
    }

    /// Removes an adapter from state entirely.
    pub fn remove_adapter(&self, _adapter_id: &str) -> Result<(), StateError> {
        todo!("remove_adapter not yet implemented")
    }

    /// Returns the currently RUNNING adapter, if any.
    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        todo!("get_running_adapter not yet implemented")
    }

    /// Returns STOPPED adapters whose stopped_at exceeds the given timeout.
    pub fn get_offload_candidates(&self, _timeout: Duration) -> Vec<AdapterEntry> {
        todo!("get_offload_candidates not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(adapter_id: &str, image_ref: &str, state: AdapterState) -> AdapterEntry {
        AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:test".to_string(),
            state,
            job_id: "test-job-id".to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    // TS-07-11: GetAdapterStatus Returns Current State
    // Requirement: 07-REQ-4.2
    #[test]
    fn test_create_and_get_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let entry = make_entry("adapter-a-v1", "example.com/adapter-a:v1", AdapterState::Running);
        mgr.create_adapter(entry);
        let got = mgr.get_adapter("adapter-a-v1").expect("adapter should exist");
        assert_eq!(got.adapter_id, "adapter-a-v1");
        assert_eq!(got.state, AdapterState::Running);
    }

    // TS-07-E9: ListAdapters Returns Empty When None Installed
    // Requirement: 07-REQ-4.E2
    #[test]
    fn test_list_adapters_empty() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let adapters = mgr.list_adapters();
        assert!(adapters.is_empty());
    }

    // TS-07-10: ListAdapters Returns All Known Adapters
    // Requirement: 07-REQ-4.1
    #[test]
    fn test_list_adapters_multiple() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry("a-v1", "img-a:v1", AdapterState::Running));
        mgr.create_adapter(make_entry("b-v2", "img-b:v2", AdapterState::Stopped));
        let adapters = mgr.list_adapters();
        assert_eq!(adapters.len(), 2);
        let mut ids: Vec<String> = adapters.iter().map(|a| a.adapter_id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["a-v1", "b-v2"]);
    }

    // TS-07-E8: GetAdapterStatus Unknown ID Returns NOT_FOUND
    // Requirement: 07-REQ-4.E1
    #[test]
    fn test_get_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        assert!(mgr.get_adapter("nonexistent-adapter").is_none());
    }

    // TS-07-12: RemoveAdapter Cleans Up (state layer)
    // Requirement: 07-REQ-5.1, 07-REQ-5.2
    #[test]
    fn test_remove_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry("adapter-rm", "img:v1", AdapterState::Stopped));
        mgr.remove_adapter("adapter-rm").expect("should succeed");
        assert!(mgr.get_adapter("adapter-rm").is_none());
    }

    // TS-07-E10: RemoveAdapter Unknown ID Returns NOT_FOUND
    // Requirement: 07-REQ-5.E1
    #[test]
    fn test_remove_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let result = mgr.remove_adapter("nonexistent-adapter");
        assert!(result.is_err());
    }

    // TS-07-8: WatchAdapterStates Streams Events
    // Requirements: 07-REQ-3.1, 07-REQ-3.2, 07-REQ-3.3, 07-REQ-8.1, 07-REQ-8.2, 07-REQ-8.3
    #[test]
    fn test_state_transition_emits_event() {
        let (tx, mut rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry(
            "ev-adapter",
            "img:v1",
            AdapterState::Unknown,
        ));
        mgr.transition("ev-adapter", AdapterState::Downloading, None)
            .expect("transition should succeed");

        let event = rx.try_recv().expect("should receive event");
        assert_eq!(event.adapter_id, "ev-adapter");
        assert_eq!(event.old_state, AdapterState::Unknown);
        assert_eq!(event.new_state, AdapterState::Downloading);
        assert!(event.timestamp > 0);
    }

    // TS-07-9: WatchAdapterStates No Historical Replay
    // Requirement: 07-REQ-3.4
    #[test]
    fn test_no_historical_replay() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx.clone());
        mgr.create_adapter(make_entry(
            "hist-adapter",
            "img:v1",
            AdapterState::Unknown,
        ));
        // Transition before subscribing.
        mgr.transition("hist-adapter", AdapterState::Downloading, None)
            .unwrap();

        // Subscribe after the transition.
        let mut rx2 = tx.subscribe();

        // Trigger a new transition.
        mgr.transition("hist-adapter", AdapterState::Installing, None)
            .unwrap();

        let event = rx2.try_recv().expect("should receive new event");
        assert_eq!(event.old_state, AdapterState::Downloading);
        assert_eq!(event.new_state, AdapterState::Installing);

        // There should not be a UNKNOWN->DOWNLOADING event for the late subscriber.
    }

    // TS-07-E15: No Subscribers Active During Transition
    // Requirement: 07-REQ-8.E1
    #[test]
    fn test_no_subscribers_no_error() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry(
            "no-sub-adapter",
            "img:v1",
            AdapterState::Unknown,
        ));
        // Drop all receivers; transition should still succeed.
        drop(_rx);
        mgr.transition("no-sub-adapter", AdapterState::Downloading, None)
            .expect("transition should succeed even with no subscribers");
        let adapter = mgr.get_adapter("no-sub-adapter").expect("should exist");
        assert_eq!(adapter.state, AdapterState::Downloading);
    }

    // TS-07-E7: Subscriber Disconnect Does Not Affect Others
    // Requirement: 07-REQ-3.E1
    #[test]
    fn test_subscriber_disconnect() {
        let (tx, _rx1) = broadcast::channel(16);
        let mut rx2 = tx.subscribe();
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry(
            "disc-adapter",
            "img:v1",
            AdapterState::Unknown,
        ));
        // Drop first subscriber.
        drop(_rx1);
        // Transition; second subscriber should still receive.
        mgr.transition("disc-adapter", AdapterState::Downloading, None)
            .expect("should succeed");
        let event = rx2.try_recv().expect("subscriber 2 should receive event");
        assert_eq!(event.adapter_id, "disc-adapter");
        assert_eq!(event.new_state, AdapterState::Downloading);
    }
}
