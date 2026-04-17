//! In-memory adapter state manager.
//!
//! Stores adapter entries in a thread-safe map and emits broadcast events on
//! every state transition (07-REQ-3.2, 07-REQ-8.1).

#![allow(dead_code)]

use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};

/// Errors returned by state-manager operations.
#[derive(Debug, PartialEq, Eq)]
pub enum StateError {
    NotFound,
    AlreadyExists,
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::NotFound => write!(f, "adapter not found"),
            StateError::AlreadyExists => write!(f, "adapter already exists"),
        }
    }
}

impl std::error::Error for StateError {}

/// Thread-safe, in-memory store of adapter entries.
///
/// All state transitions emit an [`AdapterStateEvent`] via the supplied
/// broadcast channel.
pub struct StateManager {
    broadcaster: tokio::sync::broadcast::Sender<AdapterStateEvent>,
    // Concrete storage is added in task group 3.
    _inner: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, AdapterEntry>>>,
}

impl StateManager {
    /// Create a new state manager backed by the given broadcast sender.
    pub fn new(_broadcaster: tokio::sync::broadcast::Sender<AdapterStateEvent>) -> Self {
        todo!()
    }

    /// Insert a new adapter entry. Panics (via todo!) until task group 3.
    pub fn create_adapter(&self, _entry: AdapterEntry) {
        todo!()
    }

    /// Look up an adapter by ID. Returns `None` if not found.
    pub fn get_adapter(&self, _adapter_id: &str) -> Option<AdapterEntry> {
        todo!()
    }

    /// Return all stored adapter entries.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        todo!()
    }

    /// Remove an adapter by ID. Returns `Err(StateError::NotFound)` if absent.
    pub fn remove_adapter(&self, _adapter_id: &str) -> Result<(), StateError> {
        todo!()
    }

    /// Transition an adapter to a new state, emitting a broadcast event.
    pub fn transition(
        &self,
        _adapter_id: &str,
        _new_state: AdapterState,
        _error_message: Option<String>,
    ) -> Result<(), StateError> {
        todo!()
    }

    /// Return the adapter currently in RUNNING state, if any.
    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        todo!()
    }

    /// Return adapters in STOPPED state whose `stopped_at` timestamp is older
    /// than `timeout`.
    pub fn get_offload_candidates(
        &self,
        _timeout: std::time::Duration,
    ) -> Vec<AdapterEntry> {
        todo!()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
    use tokio::sync::broadcast;

    fn make_entry(adapter_id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: AdapterState::Unknown,
            job_id: "job-1".to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    /// TS-07-11: GetAdapterStatus returns current state
    #[test]
    fn test_create_and_get_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let entry = make_entry("my-adapter-v1", "registry.io/my-adapter:v1");
        mgr.create_adapter(entry);
        let got = mgr.get_adapter("my-adapter-v1").unwrap();
        assert_eq!(got.adapter_id, "my-adapter-v1");
    }

    /// TS-07-E9: ListAdapters returns empty when none installed
    #[test]
    fn test_list_adapters_empty() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let adapters = mgr.list_adapters();
        assert!(adapters.is_empty());
    }

    /// TS-07-10: ListAdapters returns all adapters
    #[test]
    fn test_list_adapters_multiple() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry("adapter-a-v1", "reg.io/adapter-a:v1"));
        mgr.create_adapter(make_entry("adapter-b-v1", "reg.io/adapter-b:v1"));
        let adapters = mgr.list_adapters();
        assert_eq!(adapters.len(), 2);
    }

    /// TS-07-E8: GetAdapterStatus unknown ID returns error
    #[test]
    fn test_get_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        assert!(mgr.get_adapter("nonexistent").is_none());
    }

    /// TS-07-12: RemoveAdapter removes from state
    #[test]
    fn test_remove_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry("adapter-a-v1", "reg.io/adapter-a:v1"));
        mgr.remove_adapter("adapter-a-v1").unwrap();
        assert!(mgr.get_adapter("adapter-a-v1").is_none());
    }

    /// TS-07-E10: RemoveAdapter unknown ID returns error
    #[test]
    fn test_remove_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        let result = mgr.remove_adapter("nonexistent");
        assert!(result.is_err());
    }

    /// TS-07-8: state transition emits event
    #[tokio::test]
    async fn test_state_transition_emits_event() {
        let (tx, mut rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry("adapter-a-v1", "reg.io/adapter-a:v1"));
        mgr.transition("adapter-a-v1", AdapterState::Downloading, None)
            .unwrap();
        let event = rx.try_recv().unwrap();
        assert_eq!(event.adapter_id, "adapter-a-v1");
        assert_eq!(event.old_state, AdapterState::Unknown);
        assert_eq!(event.new_state, AdapterState::Downloading);
        assert!(event.timestamp > 0);
    }

    /// TS-07-9: new subscriber does not receive historical events
    #[tokio::test]
    async fn test_no_historical_replay() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx.clone());
        mgr.create_adapter(make_entry("adapter-a-v1", "reg.io/adapter-a:v1"));
        mgr.transition("adapter-a-v1", AdapterState::Downloading, None)
            .unwrap();
        // Subscribe AFTER the transition
        let mut rx2 = tx.subscribe();
        // No events should be available
        assert!(rx2.try_recv().is_err());
    }

    /// TS-07-E15: no subscribers during transition is fine
    #[test]
    fn test_no_subscribers_no_error() {
        let (tx, rx) = broadcast::channel::<AdapterStateEvent>(16);
        // Drop all receivers
        drop(rx);
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry("adapter-a-v1", "reg.io/adapter-a:v1"));
        // Should not panic
        let _ = mgr.transition("adapter-a-v1", AdapterState::Downloading, None);
    }

    /// TS-07-E7: subscriber disconnect does not affect others
    #[tokio::test]
    async fn test_subscriber_disconnect() {
        let (tx, rx1) = broadcast::channel(16);
        let mut rx2 = tx.subscribe();
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry("adapter-a-v1", "reg.io/adapter-a:v1"));
        // Drop rx1 (disconnect subscriber 1)
        drop(rx1);
        mgr.transition("adapter-a-v1", AdapterState::Downloading, None)
            .unwrap();
        // rx2 should still receive the event
        let event = rx2.try_recv().unwrap();
        assert_eq!(event.new_state, AdapterState::Downloading);
    }

    /// TS-07-P3: state transition validity (property test scaffold)
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_state_transition_validity() {
        // Implemented in task group 3.
        todo!()
    }

    /// TS-07-P4: event delivery completeness (property test scaffold)
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_event_delivery_completeness() {
        // Implemented in task group 3.
        todo!()
    }
}
