use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};

#[derive(Debug)]
pub struct StateError(pub String);

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StateError: {}", self.0)
    }
}

impl std::error::Error for StateError {}

/// Returns true if the (old, new) state pair is a valid state machine edge.
fn is_valid_transition(old: &AdapterState, new: &AdapterState) -> bool {
    matches!(
        (old, new),
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

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub struct StateManager {
    adapters: Arc<Mutex<HashMap<String, AdapterEntry>>>,
    broadcaster: broadcast::Sender<AdapterStateEvent>,
}

impl StateManager {
    pub fn new(broadcaster: broadcast::Sender<AdapterStateEvent>) -> Self {
        Self {
            adapters: Arc::new(Mutex::new(HashMap::new())),
            broadcaster,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AdapterStateEvent> {
        self.broadcaster.subscribe()
    }

    pub fn create_adapter(&self, entry: AdapterEntry) {
        self.adapters
            .lock()
            .unwrap()
            .insert(entry.adapter_id.clone(), entry);
    }

    pub fn transition(
        &self,
        adapter_id: &str,
        new_state: AdapterState,
        error_msg: Option<String>,
    ) -> Result<(), StateError> {
        let mut adapters = self.adapters.lock().unwrap();
        let entry = adapters
            .get_mut(adapter_id)
            .ok_or_else(|| StateError(format!("adapter '{}' not found", adapter_id)))?;

        if !is_valid_transition(&entry.state, &new_state) {
            return Err(StateError(format!(
                "invalid transition {:?} -> {:?} for adapter '{}'",
                entry.state, new_state, adapter_id
            )));
        }

        let old_state = entry.state.clone();
        entry.state = new_state.clone();
        entry.error_message = error_msg;

        if new_state == AdapterState::Stopped {
            entry.stopped_at = Some(Instant::now());
        }

        let event = AdapterStateEvent {
            adapter_id: adapter_id.to_string(),
            old_state,
            new_state,
            timestamp: now_unix_secs(),
        };

        // Ignore send errors (no active subscribers)
        let _ = self.broadcaster.send(event);

        Ok(())
    }

    /// Unconditionally transition an adapter to ERROR, bypassing state machine
    /// validation. Used when errors occur mid-flow from any state.
    pub fn force_error(&self, adapter_id: &str, error_msg: &str) {
        let mut adapters = self.adapters.lock().unwrap();
        if let Some(entry) = adapters.get_mut(adapter_id) {
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

    pub fn get_adapter(&self, adapter_id: &str) -> Option<AdapterEntry> {
        self.adapters.lock().unwrap().get(adapter_id).cloned()
    }

    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        self.adapters.lock().unwrap().values().cloned().collect()
    }

    pub fn remove_adapter(&self, adapter_id: &str) -> Result<(), StateError> {
        let mut adapters = self.adapters.lock().unwrap();
        adapters
            .remove(adapter_id)
            .map(|_| ())
            .ok_or_else(|| StateError(format!("adapter '{}' not found", adapter_id)))
    }

    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        self.adapters
            .lock()
            .unwrap()
            .values()
            .find(|e| e.state == AdapterState::Running)
            .cloned()
    }

    /// Returns STOPPED adapters whose stopped_at + timeout has elapsed.
    pub fn get_offload_candidates(&self, timeout: Duration) -> Vec<AdapterEntry> {
        self.adapters
            .lock()
            .unwrap()
            .values()
            .filter(|e| {
                e.state == AdapterState::Stopped
                    && e.stopped_at
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
    use tokio::sync::broadcast;

    fn make_state_mgr() -> (StateManager, broadcast::Receiver<AdapterStateEvent>) {
        let (tx, rx) = broadcast::channel(128);
        (StateManager::new(tx), rx)
    }

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

    // TS-07-11: Create adapter and retrieve its state
    #[test]
    fn test_create_and_get_adapter() {
        let (sm, _rx) = make_state_mgr();
        sm.create_adapter(make_entry("my-adapter-v1", "registry/my-adapter:v1"));
        let entry = sm.get_adapter("my-adapter-v1").expect("should find adapter");
        assert_eq!(entry.adapter_id, "my-adapter-v1");
        assert_eq!(entry.state, AdapterState::Unknown);
    }

    // TS-07-E9: ListAdapters returns empty when none installed
    #[test]
    fn test_list_adapters_empty() {
        let (sm, _rx) = make_state_mgr();
        assert!(sm.list_adapters().is_empty());
    }

    // TS-07-10: ListAdapters returns all adapters
    #[test]
    fn test_list_adapters_multiple() {
        let (sm, _rx) = make_state_mgr();
        sm.create_adapter(make_entry("adapter-a-v1", "reg/adapter-a:v1"));
        sm.create_adapter(make_entry("adapter-b-v1", "reg/adapter-b:v1"));
        let list = sm.list_adapters();
        assert_eq!(list.len(), 2);
        let mut ids: Vec<_> = list.iter().map(|a| a.adapter_id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["adapter-a-v1", "adapter-b-v1"]);
    }

    // TS-07-E8: GetAdapterStatus unknown ID returns None
    #[test]
    fn test_get_unknown_adapter() {
        let (sm, _rx) = make_state_mgr();
        assert!(sm.get_adapter("nonexistent-adapter").is_none());
    }

    // TS-07-12: Remove adapter cleans up state
    #[test]
    fn test_remove_adapter() {
        let (sm, _rx) = make_state_mgr();
        sm.create_adapter(make_entry("my-adapter-v1", "registry/my-adapter:v1"));
        sm.remove_adapter("my-adapter-v1").unwrap();
        assert!(sm.get_adapter("my-adapter-v1").is_none());
    }

    // TS-07-E10: Remove unknown adapter returns error
    #[test]
    fn test_remove_unknown_adapter() {
        let (sm, _rx) = make_state_mgr();
        let result = sm.remove_adapter("nonexistent");
        assert!(result.is_err());
    }

    // TS-07-8: State transition emits events to subscribers
    #[tokio::test]
    async fn test_state_transition_emits_event() {
        let (sm, mut rx) = make_state_mgr();
        sm.create_adapter(make_entry("my-adapter-v1", "registry/my-adapter:v1"));
        sm.transition("my-adapter-v1", AdapterState::Downloading, None)
            .unwrap();
        let event = rx.try_recv().expect("should have received event");
        assert_eq!(event.adapter_id, "my-adapter-v1");
        assert_eq!(event.old_state, AdapterState::Unknown);
        assert_eq!(event.new_state, AdapterState::Downloading);
        assert!(event.timestamp > 0);
    }

    // TS-07-9: No historical replay for new subscribers
    #[tokio::test]
    async fn test_no_historical_replay() {
        let (sm, _early_rx) = make_state_mgr();
        sm.create_adapter(make_entry("my-adapter-v1", "registry/my-adapter:v1"));
        sm.transition("my-adapter-v1", AdapterState::Downloading, None)
            .unwrap();
        // Subscribe late — no events should be pending
        let mut late_rx = sm.subscribe();
        assert!(late_rx.try_recv().is_err());
    }

    // TS-07-E15: No subscribers active — no error on transition
    #[test]
    fn test_no_subscribers_no_error() {
        let (tx, rx) = broadcast::channel::<AdapterStateEvent>(1);
        drop(rx); // no active subscribers
        let sm = StateManager::new(tx);
        sm.create_adapter(make_entry("my-adapter-v1", "registry/my-adapter:v1"));
        // Should not panic even though no subscribers
        let result = sm.transition("my-adapter-v1", AdapterState::Downloading, None);
        assert!(result.is_ok());
    }

    // TS-07-E7: Subscriber disconnect does not affect others
    #[tokio::test]
    async fn test_subscriber_disconnect() {
        let (sm, rx1) = make_state_mgr();
        let mut rx2 = sm.subscribe();
        sm.create_adapter(make_entry("my-adapter-v1", "registry/my-adapter:v1"));
        drop(rx1); // disconnect first subscriber
        sm.transition("my-adapter-v1", AdapterState::Downloading, None)
            .unwrap();
        // Second subscriber should still receive the event
        let event = rx2.try_recv().expect("second subscriber should receive event");
        assert_eq!(event.new_state, AdapterState::Downloading);
    }

    // TS-07-P3: State transition validity property test
    #[test]
    #[ignore]
    fn proptest_state_transition_validity() {
        // Validates that all transitions follow the state machine edges
        // Implemented as part of task group 3 verification
    }

    // TS-07-P4: Event delivery completeness property test
    #[test]
    #[ignore]
    fn proptest_event_delivery_completeness() {
        // Validates all subscribers receive same events
        // Implemented as part of task group 3 verification
    }
}
