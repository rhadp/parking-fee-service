use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

/// Errors from state management operations.
#[derive(Debug)]
pub enum StateError {
    NotFound(String),
    InvalidTransition(String),
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::NotFound(msg) => write!(f, "not found: {msg}"),
            StateError::InvalidTransition(msg) => write!(f, "invalid transition: {msg}"),
        }
    }
}

impl std::error::Error for StateError {}

/// Thread-safe in-memory adapter state store with event broadcasting.
pub struct StateManager {
    tx: broadcast::Sender<AdapterStateEvent>,
    adapters: Mutex<HashMap<String, AdapterEntry>>,
}

impl StateManager {
    /// Create a new state manager with the given broadcast sender.
    pub fn new(tx: broadcast::Sender<AdapterStateEvent>) -> Self {
        Self {
            tx,
            adapters: Mutex::new(HashMap::new()),
        }
    }

    /// Subscribe to adapter state events.
    pub fn subscribe(&self) -> broadcast::Receiver<AdapterStateEvent> {
        self.tx.subscribe()
    }

    /// Insert a new adapter entry.
    pub fn create_adapter(&self, entry: AdapterEntry) {
        let mut map = self.adapters.lock().unwrap();
        map.insert(entry.adapter_id.clone(), entry);
    }

    /// Transition an adapter to a new state, emitting an event.
    pub fn transition(
        &self,
        adapter_id: &str,
        new_state: AdapterState,
        error_msg: Option<String>,
    ) -> Result<(), StateError> {
        let mut map = self.adapters.lock().unwrap();
        let entry = map
            .get_mut(adapter_id)
            .ok_or_else(|| StateError::NotFound(format!("adapter {adapter_id} not found")))?;

        let old_state = entry.state.clone();
        entry.state = new_state.clone();

        // Record stopped_at when transitioning to Stopped
        if new_state == AdapterState::Stopped {
            entry.stopped_at = Some(Instant::now());
        }

        // Record error message
        if let Some(msg) = error_msg {
            entry.error_message = Some(msg);
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let event = AdapterStateEvent {
            adapter_id: adapter_id.to_string(),
            old_state,
            new_state,
            timestamp,
        };

        // Ignore send errors (no subscribers is fine per 07-REQ-8.E1)
        let _ = self.tx.send(event);

        Ok(())
    }

    /// Look up an adapter by ID.
    pub fn get_adapter(&self, adapter_id: &str) -> Option<AdapterEntry> {
        let map = self.adapters.lock().unwrap();
        map.get(adapter_id).cloned()
    }

    /// List all known adapters.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        let map = self.adapters.lock().unwrap();
        map.values().cloned().collect()
    }

    /// Remove an adapter from state entirely.
    pub fn remove_adapter(&self, adapter_id: &str) -> Result<(), StateError> {
        let mut map = self.adapters.lock().unwrap();
        if map.remove(adapter_id).is_some() {
            Ok(())
        } else {
            Err(StateError::NotFound(format!(
                "adapter {adapter_id} not found"
            )))
        }
    }

    /// Return the currently RUNNING adapter, if any.
    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        let map = self.adapters.lock().unwrap();
        map.values()
            .find(|e| e.state == AdapterState::Running)
            .cloned()
    }

    /// Return STOPPED adapters whose inactivity exceeds `timeout`.
    pub fn get_offload_candidates(&self, timeout: Duration) -> Vec<AdapterEntry> {
        let map = self.adapters.lock().unwrap();
        let now = Instant::now();
        map.values()
            .filter(|e| {
                e.state == AdapterState::Stopped
                    && e.stopped_at
                        .map(|t| now.duration_since(t) >= timeout)
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
    use tokio::sync::broadcast;

    fn make_entry(adapter_id: &str, image_ref: &str) -> AdapterEntry {
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

    // TS-07-11: GetAdapterStatus Returns Current State
    #[test]
    fn test_create_and_get_adapter() {
        let (tx, _rx) = broadcast::channel::<AdapterStateEvent>(16);
        let mgr = StateManager::new(tx);
        let entry = make_entry("parkhaus-munich-v1.0.0", "example.com/parkhaus-munich:v1.0.0");

        mgr.create_adapter(entry);

        let found = mgr.get_adapter("parkhaus-munich-v1.0.0");
        assert!(found.is_some(), "adapter should be found after creation");
        let found = found.unwrap();
        assert_eq!(found.adapter_id, "parkhaus-munich-v1.0.0");
        assert_eq!(found.state, AdapterState::Unknown);
    }

    // TS-07-E9: ListAdapters Returns Empty When None Installed
    #[test]
    fn test_list_adapters_empty() {
        let (tx, _rx) = broadcast::channel::<AdapterStateEvent>(16);
        let mgr = StateManager::new(tx);

        let adapters = mgr.list_adapters();
        assert_eq!(adapters.len(), 0, "should return empty list");
    }

    // TS-07-10: ListAdapters Returns All Known Adapters
    #[test]
    fn test_list_adapters_multiple() {
        let (tx, _rx) = broadcast::channel::<AdapterStateEvent>(16);
        let mgr = StateManager::new(tx);

        mgr.create_adapter(make_entry("adapter-a-v1", "example.com/adapter-a:v1"));
        mgr.create_adapter(make_entry("adapter-b-v2", "example.com/adapter-b:v2"));

        let adapters = mgr.list_adapters();
        assert_eq!(adapters.len(), 2);
        let mut ids: Vec<String> = adapters.iter().map(|a| a.adapter_id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["adapter-a-v1", "adapter-b-v2"]);
    }

    // TS-07-E8: GetAdapterStatus Unknown ID
    #[test]
    fn test_get_unknown_adapter() {
        let (tx, _rx) = broadcast::channel::<AdapterStateEvent>(16);
        let mgr = StateManager::new(tx);

        let result = mgr.get_adapter("nonexistent-adapter");
        assert!(result.is_none(), "unknown adapter should return None");
    }

    // TS-07-12: RemoveAdapter (state-level test)
    #[test]
    fn test_remove_adapter() {
        let (tx, _rx) = broadcast::channel::<AdapterStateEvent>(16);
        let mgr = StateManager::new(tx);
        mgr.create_adapter(make_entry(
            "parkhaus-munich-v1.0.0",
            "example.com/parkhaus-munich:v1.0.0",
        ));

        let result = mgr.remove_adapter("parkhaus-munich-v1.0.0");
        assert!(result.is_ok(), "remove should succeed");

        let found = mgr.get_adapter("parkhaus-munich-v1.0.0");
        assert!(found.is_none(), "adapter should be gone after removal");
    }

    // TS-07-E10: RemoveAdapter Unknown ID
    #[test]
    fn test_remove_unknown_adapter() {
        let (tx, _rx) = broadcast::channel::<AdapterStateEvent>(16);
        let mgr = StateManager::new(tx);

        let result = mgr.remove_adapter("nonexistent");
        assert!(result.is_err(), "removing unknown adapter should error");
    }

    // TS-07-8: State Transition Emits Event
    #[tokio::test]
    async fn test_state_transition_emits_event() {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
        let mgr = StateManager::new(tx);
        let mut rx = mgr.subscribe();

        mgr.create_adapter(make_entry(
            "parkhaus-munich-v1.0.0",
            "example.com/parkhaus-munich:v1.0.0",
        ));

        // Transition UNKNOWN -> DOWNLOADING
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Downloading, None)
            .unwrap();

        let event = rx.try_recv().expect("should receive event");
        assert_eq!(event.adapter_id, "parkhaus-munich-v1.0.0");
        assert_eq!(event.old_state, AdapterState::Unknown);
        assert_eq!(event.new_state, AdapterState::Downloading);
        assert!(event.timestamp > 0, "timestamp should be set");

        // Transition DOWNLOADING -> INSTALLING
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Installing, None)
            .unwrap();

        let event = rx.try_recv().expect("should receive second event");
        assert_eq!(event.old_state, AdapterState::Downloading);
        assert_eq!(event.new_state, AdapterState::Installing);

        // Transition INSTALLING -> RUNNING
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Running, None)
            .unwrap();

        let event = rx.try_recv().expect("should receive third event");
        assert_eq!(event.old_state, AdapterState::Installing);
        assert_eq!(event.new_state, AdapterState::Running);
    }

    // TS-07-9: WatchAdapterStates No Historical Replay
    #[tokio::test]
    async fn test_no_historical_replay() {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
        let mgr = StateManager::new(tx);

        // Perform transitions BEFORE subscribing
        mgr.create_adapter(make_entry(
            "parkhaus-munich-v1.0.0",
            "example.com/parkhaus-munich:v1.0.0",
        ));
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Running, None)
            .unwrap();

        // Subscribe AFTER transitions
        let mut rx = mgr.subscribe();

        // Trigger a new transition
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Stopped, None)
            .unwrap();

        // Should only see RUNNING->STOPPED, not earlier events
        let event = rx.try_recv().expect("should receive new event");
        assert_eq!(event.old_state, AdapterState::Running);
        assert_eq!(event.new_state, AdapterState::Stopped);

        // No more events
        assert!(
            rx.try_recv().is_err(),
            "should not have received historical events"
        );
    }

    // TS-07-E15: No Subscribers Active During Transition
    #[test]
    fn test_no_subscribers_no_error() {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
        // Drop the initial receiver so there are zero subscribers
        let mgr = StateManager::new(tx);

        mgr.create_adapter(make_entry(
            "parkhaus-munich-v1.0.0",
            "example.com/parkhaus-munich:v1.0.0",
        ));

        // This should not panic even with no subscribers
        let result =
            mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Downloading, None);
        assert!(
            result.is_ok(),
            "transition should succeed with no subscribers"
        );
    }

    // TS-07-E7: Subscriber Disconnect Does Not Affect Others
    #[tokio::test]
    async fn test_subscriber_disconnect() {
        let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
        let mgr = StateManager::new(tx);

        let rx1 = mgr.subscribe();
        let mut rx2 = mgr.subscribe();

        // Disconnect subscriber 1
        drop(rx1);

        mgr.create_adapter(make_entry(
            "parkhaus-munich-v1.0.0",
            "example.com/parkhaus-munich:v1.0.0",
        ));
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Downloading, None)
            .unwrap();

        // Subscriber 2 should still receive the event
        let event = rx2.try_recv().expect("subscriber 2 should receive event");
        assert_eq!(event.new_state, AdapterState::Downloading);
    }
}
