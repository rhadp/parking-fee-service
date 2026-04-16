use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
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
    #[allow(dead_code)]
    inner: Mutex<HashMap<String, AdapterEntry>>,
    broadcaster: broadcast::Sender<AdapterStateEvent>,
}

impl StateManager {
    pub fn new(broadcaster: broadcast::Sender<AdapterStateEvent>) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            broadcaster,
        }
    }

    pub fn create_adapter(&self, _entry: AdapterEntry) {
        todo!("implement create_adapter")
    }

    pub fn transition(
        &self,
        _adapter_id: &str,
        _new_state: AdapterState,
        _error_msg: Option<String>,
    ) -> Result<(), StateError> {
        todo!("implement transition")
    }

    pub fn get_adapter(&self, _adapter_id: &str) -> Option<AdapterEntry> {
        todo!("implement get_adapter")
    }

    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        todo!("implement list_adapters")
    }

    pub fn remove_adapter(&self, _adapter_id: &str) -> Result<(), StateError> {
        todo!("implement remove_adapter")
    }

    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        todo!("implement get_running_adapter")
    }

    pub fn get_offload_candidates(&self, _timeout: Duration) -> Vec<AdapterEntry> {
        todo!("implement get_offload_candidates")
    }

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
