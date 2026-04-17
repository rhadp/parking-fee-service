//! In-memory adapter state manager.
//!
//! Stores adapter entries in a thread-safe map and emits broadcast events on
//! every state transition (07-REQ-3.2, 07-REQ-8.1).

#![allow(dead_code)]

use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
    inner: Arc<Mutex<HashMap<String, AdapterEntry>>>,
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl StateManager {
    /// Create a new state manager backed by the given broadcast sender.
    pub fn new(broadcaster: tokio::sync::broadcast::Sender<AdapterStateEvent>) -> Self {
        Self {
            broadcaster,
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Insert a new adapter entry.
    pub fn create_adapter(&self, entry: AdapterEntry) {
        let mut map = self.inner.lock().unwrap();
        map.insert(entry.adapter_id.clone(), entry);
    }

    /// Look up an adapter by ID. Returns `None` if not found.
    pub fn get_adapter(&self, adapter_id: &str) -> Option<AdapterEntry> {
        self.inner.lock().unwrap().get(adapter_id).cloned()
    }

    /// Return all stored adapter entries.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        self.inner.lock().unwrap().values().cloned().collect()
    }

    /// Remove an adapter by ID. Returns `Err(StateError::NotFound)` if absent.
    pub fn remove_adapter(&self, adapter_id: &str) -> Result<(), StateError> {
        let mut map = self.inner.lock().unwrap();
        if map.remove(adapter_id).is_none() {
            return Err(StateError::NotFound);
        }
        Ok(())
    }

    /// Transition an adapter to a new state, emitting a broadcast event.
    ///
    /// The lock is released before broadcasting to avoid potential deadlocks
    /// when broadcast receivers are driven within the same task.
    pub fn transition(
        &self,
        adapter_id: &str,
        new_state: AdapterState,
        error_message: Option<String>,
    ) -> Result<(), StateError> {
        let event = {
            let mut map = self.inner.lock().unwrap();
            let entry = map.get_mut(adapter_id).ok_or(StateError::NotFound)?;

            let old_state = entry.state.clone();
            entry.state = new_state.clone();
            entry.error_message = error_message;

            // Record when an adapter becomes STOPPED (for offload timer).
            if new_state == AdapterState::Stopped {
                entry.stopped_at = Some(Instant::now());
            }

            AdapterStateEvent {
                adapter_id: adapter_id.to_string(),
                old_state,
                new_state,
                timestamp: unix_millis(),
            }
        }; // lock released here

        let _ = self.broadcaster.send(event);
        Ok(())
    }

    /// Forcibly set an adapter's state to ERROR, bypassing transition validation.
    ///
    /// Used when error recording must happen regardless of the current state
    /// (e.g., after a failed podman operation).
    pub fn force_error(&self, adapter_id: &str, error_message: String) {
        let event = {
            let mut map = self.inner.lock().unwrap();
            let Some(entry) = map.get_mut(adapter_id) else {
                return;
            };

            let old_state = entry.state.clone();
            entry.state = AdapterState::Error;
            entry.error_message = Some(error_message);

            Some(AdapterStateEvent {
                adapter_id: adapter_id.to_string(),
                old_state,
                new_state: AdapterState::Error,
                timestamp: unix_millis(),
            })
        }; // lock released here

        if let Some(ev) = event {
            let _ = self.broadcaster.send(ev);
        }
    }

    /// Return the adapter currently in RUNNING state, if any.
    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        self.inner
            .lock()
            .unwrap()
            .values()
            .find(|e| e.state == AdapterState::Running)
            .cloned()
    }

    /// Return adapters in STOPPED state whose `stopped_at` timestamp is older
    /// than `timeout`.
    pub fn get_offload_candidates(&self, timeout: Duration) -> Vec<AdapterEntry> {
        let now = Instant::now();
        self.inner
            .lock()
            .unwrap()
            .values()
            .filter(|e| {
                e.state == AdapterState::Stopped
                    && e.stopped_at
                        .is_some_and(|t| now.duration_since(t) >= timeout)
            })
            .cloned()
            .collect()
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
        use proptest::prelude::*;

        fn is_valid(from: &AdapterState, to: &AdapterState) -> bool {
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
                    | (AdapterState::Stopped, AdapterState::Running)
                    | (AdapterState::Offloading, AdapterState::Error)
            )
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(seed in 0usize..3)| {
            use crate::podman::{MockPodmanExecutor, PodmanError, UpdateServiceImpl};
            use std::sync::Arc;

            let image_ref = format!("reg.io/adapter-{seed}:v1");
            let checksum = format!("sha256:abc{seed:060}");

            let events = rt.block_on(async {
                let (tx, mut rx) = broadcast::channel::<AdapterStateEvent>(64);
                let state = Arc::new(StateManager::new(tx.clone()));
                let mock = Arc::new(MockPodmanExecutor::new());
                mock.set_pull_result(Ok(()));
                mock.set_inspect_result(Ok(checksum.clone()));
                mock.set_run_result(Ok(()));
                mock.set_wait_result(Err(PodmanError::new("block")));

                let svc = UpdateServiceImpl::new(Arc::clone(&state), Arc::clone(&mock), tx);
                let _ = svc.install_adapter(&image_ref, &checksum).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

                let mut events = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    events.push((event.old_state, event.new_state));
                }
                events
            });

            for (old, new) in &events {
                prop_assert!(
                    is_valid(old, new),
                    "Invalid transition: {:?} -> {:?}", old, new
                );
            }
        });
    }

    /// TS-07-P4: event delivery completeness (property test scaffold)
    #[test]
    #[ignore = "proptest: run with --include-ignored proptest"]
    fn proptest_event_delivery_completeness() {
        use proptest::prelude::*;

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(n in 1usize..=3)| {
            use crate::podman::{MockPodmanExecutor, PodmanError, UpdateServiceImpl};
            use std::sync::Arc;

            let image_ref = "reg.io/adapter-a:v1";
            let checksum = "sha256:abcdef";

            let all_events = rt.block_on(async {
                let (tx, _rx) = broadcast::channel::<AdapterStateEvent>(64);
                let state = Arc::new(StateManager::new(tx.clone()));

                // Subscribe N receivers BEFORE install to capture all events.
                let mut receivers: Vec<_> = (0..n).map(|_| tx.subscribe()).collect();

                let mock = Arc::new(MockPodmanExecutor::new());
                mock.set_pull_result(Ok(()));
                mock.set_inspect_result(Ok(checksum.to_string()));
                mock.set_run_result(Ok(()));
                mock.set_wait_result(Err(PodmanError::new("block")));

                let svc = UpdateServiceImpl::new(Arc::clone(&state), Arc::clone(&mock), tx);
                let _ = svc.install_adapter(image_ref, checksum).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

                receivers
                    .iter_mut()
                    .map(|rx| {
                        let mut evs = Vec::new();
                        while let Ok(event) = rx.try_recv() {
                            evs.push((event.old_state, event.new_state));
                        }
                        evs
                    })
                    .collect::<Vec<_>>()
            });

            // All N subscribers should have received the same events.
            for i in 1..n {
                prop_assert_eq!(
                    &all_events[0], &all_events[i],
                    "subscriber {} received different events than subscriber 0", i
                );
            }
        });
    }
}
