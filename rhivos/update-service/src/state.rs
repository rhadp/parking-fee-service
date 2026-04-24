use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

/// Returns true if the transition from `from` to `to` is a valid state machine edge.
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
            | (AdapterState::Stopped, AdapterState::Offloading)
            | (AdapterState::Stopped, AdapterState::Error)
            | (AdapterState::Offloading, AdapterState::Error)
    )
}

/// Returns the current Unix timestamp in seconds.
fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Thread-safe in-memory state manager for adapter entries.
///
/// Emits `AdapterStateEvent` messages via a broadcast channel on every
/// state transition.
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
    pub fn create_adapter(&self, entry: AdapterEntry) {
        let mut adapters = self.adapters.lock().unwrap();
        adapters.insert(entry.adapter_id.clone(), entry);
    }

    /// Transitions the adapter to a new state and emits an event.
    pub fn transition(
        &self,
        adapter_id: &str,
        new_state: AdapterState,
        error_msg: Option<String>,
    ) -> Result<(), StateError> {
        let mut adapters = self.adapters.lock().unwrap();
        let entry = adapters
            .get_mut(adapter_id)
            .ok_or_else(|| StateError::NotFound(format!("adapter not found: {adapter_id}")))?;

        if !is_valid_transition(&entry.state, &new_state) {
            return Err(StateError::Internal(format!(
                "invalid transition: {:?} -> {:?}",
                entry.state, new_state
            )));
        }

        let old_state = entry.state.clone();
        entry.state = new_state.clone();

        // Record stopped_at timestamp when transitioning to STOPPED
        if new_state == AdapterState::Stopped {
            entry.stopped_at = Some(Instant::now());
        }

        // Record error message when transitioning to ERROR
        if new_state == AdapterState::Error {
            entry.error_message = error_msg;
        }

        let event = AdapterStateEvent {
            adapter_id: adapter_id.to_string(),
            old_state,
            new_state,
            timestamp: unix_timestamp(),
        };

        // Send event; ignore error if no receivers are active
        let _ = self.broadcaster.send(event);

        Ok(())
    }

    /// Returns a clone of the adapter entry if found.
    pub fn get_adapter(&self, adapter_id: &str) -> Option<AdapterEntry> {
        let adapters = self.adapters.lock().unwrap();
        adapters.get(adapter_id).cloned()
    }

    /// Returns a list of all known adapter entries.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        let adapters = self.adapters.lock().unwrap();
        adapters.values().cloned().collect()
    }

    /// Removes an adapter from in-memory state.
    pub fn remove_adapter(&self, adapter_id: &str) -> Result<(), StateError> {
        let mut adapters = self.adapters.lock().unwrap();
        if adapters.remove(adapter_id).is_none() {
            return Err(StateError::NotFound(format!(
                "adapter not found: {adapter_id}"
            )));
        }
        Ok(())
    }

    /// Returns the adapter currently in RUNNING state, if any.
    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        let adapters = self.adapters.lock().unwrap();
        adapters
            .values()
            .find(|a| a.state == AdapterState::Running)
            .cloned()
    }

    /// Returns adapters that have been STOPPED longer than `timeout`.
    pub fn get_offload_candidates(&self, timeout: Duration) -> Vec<AdapterEntry> {
        let adapters = self.adapters.lock().unwrap();
        adapters
            .values()
            .filter(|a| {
                a.state == AdapterState::Stopped
                    && a.stopped_at
                        .map(|t| t.elapsed() >= timeout)
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
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
