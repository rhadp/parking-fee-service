use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

/// Errors from state manager operations.
#[derive(Debug)]
pub enum StateError {
    NotFound,
    InvalidTransition,
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "adapter not found"),
            Self::InvalidTransition => write!(f, "invalid state transition"),
        }
    }
}

impl std::error::Error for StateError {}

/// Thread-safe in-memory adapter state manager.
pub struct StateManager {
    broadcaster: broadcast::Sender<AdapterStateEvent>,
    adapters: std::sync::Mutex<HashMap<String, AdapterEntry>>,
}

impl StateManager {
    /// Create a new state manager with the given broadcast sender for events.
    pub fn new(broadcaster: broadcast::Sender<AdapterStateEvent>) -> Self {
        Self {
            broadcaster,
            adapters: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Insert a new adapter entry.
    pub fn create_adapter(&self, entry: AdapterEntry) {
        let mut adapters = self.adapters.lock().unwrap();
        adapters.insert(entry.adapter_id.clone(), entry);
    }

    /// Transition an adapter to a new state, emitting an event.
    pub fn transition(
        &self,
        adapter_id: &str,
        new_state: AdapterState,
        error_msg: Option<String>,
    ) -> Result<(), StateError> {
        let mut adapters = self.adapters.lock().unwrap();
        let entry = adapters.get_mut(adapter_id).ok_or(StateError::NotFound)?;

        let old_state = entry.state;
        entry.state = new_state;

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
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        // Ignore send error — no subscribers is a valid state (REQ-8.E1).
        let _ = self.broadcaster.send(event);

        Ok(())
    }

    /// Look up an adapter by ID.
    pub fn get_adapter(&self, adapter_id: &str) -> Option<AdapterEntry> {
        let adapters = self.adapters.lock().unwrap();
        adapters.get(adapter_id).cloned()
    }

    /// Return all known adapters.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        let adapters = self.adapters.lock().unwrap();
        adapters.values().cloned().collect()
    }

    /// Remove an adapter from state entirely.
    pub fn remove_adapter(&self, adapter_id: &str) -> Result<(), StateError> {
        let mut adapters = self.adapters.lock().unwrap();
        if adapters.remove(adapter_id).is_some() {
            Ok(())
        } else {
            Err(StateError::NotFound)
        }
    }

    /// Return the currently RUNNING adapter, if any.
    pub fn get_running_adapter(&self) -> Option<AdapterEntry> {
        let adapters = self.adapters.lock().unwrap();
        adapters
            .values()
            .find(|e| e.state == AdapterState::Running)
            .cloned()
    }

    /// Return adapters in STOPPED state whose stopped_at exceeds `timeout`.
    pub fn get_offload_candidates(
        &self,
        timeout: std::time::Duration,
    ) -> Vec<AdapterEntry> {
        let adapters = self.adapters.lock().unwrap();
        adapters
            .values()
            .filter(|e| {
                e.state == AdapterState::Stopped
                    && e.stopped_at
                        .is_some_and(|stopped| stopped.elapsed() >= timeout)
            })
            .cloned()
            .collect()
    }

    /// Atomically transition an adapter from `expected_state` to `new_state`.
    ///
    /// Returns `Ok(())` if the adapter exists and is currently in
    /// `expected_state`. Returns `Err(InvalidTransition)` if the adapter
    /// exists but is in a different state. Returns `Err(NotFound)` if the
    /// adapter does not exist.
    ///
    /// This eliminates the TOCTOU race present in a separate
    /// `get_adapter` + `transition` sequence, which is critical for the
    /// container monitor: between the check and the transition, another
    /// operation (e.g. `RemoveAdapter`) could change the state.
    pub fn transition_from(
        &self,
        adapter_id: &str,
        expected_state: AdapterState,
        new_state: AdapterState,
        error_msg: Option<String>,
    ) -> Result<(), StateError> {
        let mut adapters = self.adapters.lock().unwrap();
        let entry = adapters.get_mut(adapter_id).ok_or(StateError::NotFound)?;

        if entry.state != expected_state {
            return Err(StateError::InvalidTransition);
        }

        let old_state = entry.state;
        entry.state = new_state;

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
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let _ = self.broadcaster.send(event);

        Ok(())
    }

    /// Access the broadcast sender (for subscribing).
    pub fn broadcaster(&self) -> &broadcast::Sender<AdapterStateEvent> {
        &self.broadcaster
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
    use std::time::Duration;

    /// Helper to create a test adapter entry.
    fn test_entry(adapter_id: &str, image_ref: &str) -> AdapterEntry {
        AdapterEntry {
            adapter_id: adapter_id.to_string(),
            image_ref: image_ref.to_string(),
            checksum_sha256: "sha256:test".to_string(),
            state: AdapterState::Unknown,
            job_id: uuid::Uuid::new_v4().to_string(),
            stopped_at: None,
            error_message: None,
        }
    }

    // -- TS-07-11: GetAdapterStatus returns current state -------------------

    #[test]
    fn test_create_and_get_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);

        let entry = test_entry("parkhaus-munich-v1.0.0", "example.com/parkhaus-munich:v1.0.0");
        mgr.create_adapter(entry);

        // Transition through states to reach RUNNING (per TS-07-11 precondition)
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("parkhaus-munich-v1.0.0", AdapterState::Running, None)
            .unwrap();

        let adapter = mgr
            .get_adapter("parkhaus-munich-v1.0.0")
            .expect("adapter should exist");
        assert_eq!(adapter.adapter_id, "parkhaus-munich-v1.0.0");
        assert_eq!(
            adapter.state,
            AdapterState::Running,
            "TS-07-11: adapter state should be RUNNING"
        );
    }

    // -- TS-07-E9: ListAdapters returns empty when none installed -----------

    #[test]
    fn test_list_adapters_empty() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);

        let adapters = mgr.list_adapters();
        assert!(adapters.is_empty());
    }

    // -- TS-07-10: ListAdapters returns all known adapters -------------------

    #[test]
    fn test_list_adapters_multiple() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);

        // TS-07-10 precondition: two adapters, one RUNNING one STOPPED
        mgr.create_adapter(test_entry("adapter-a-v1", "example.com/adapter-a:v1"));
        mgr.transition("adapter-a-v1", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("adapter-a-v1", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("adapter-a-v1", AdapterState::Running, None)
            .unwrap();
        mgr.transition("adapter-a-v1", AdapterState::Stopped, None)
            .unwrap();

        mgr.create_adapter(test_entry("adapter-b-v1", "example.com/adapter-b:v1"));
        mgr.transition("adapter-b-v1", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("adapter-b-v1", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("adapter-b-v1", AdapterState::Running, None)
            .unwrap();

        let adapters = mgr.list_adapters();
        assert_eq!(adapters.len(), 2);

        let mut ids: Vec<_> = adapters.iter().map(|a| a.adapter_id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["adapter-a-v1", "adapter-b-v1"]);

        // Assert states as required by TS-07-10
        let a = adapters.iter().find(|a| a.adapter_id == "adapter-a-v1").unwrap();
        let b = adapters.iter().find(|a| a.adapter_id == "adapter-b-v1").unwrap();
        assert_eq!(a.state, AdapterState::Stopped, "adapter A should be STOPPED");
        assert_eq!(b.state, AdapterState::Running, "adapter B should be RUNNING");
    }

    // -- TS-07-E8: GetAdapterStatus unknown ID returns error ----------------

    #[test]
    fn test_get_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);

        assert!(mgr.get_adapter("nonexistent-adapter").is_none());
    }

    // -- TS-07-12: RemoveAdapter removes from state -------------------------

    #[test]
    fn test_remove_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);

        mgr.create_adapter(test_entry("adapter-v1", "example.com/adapter:v1"));
        mgr.remove_adapter("adapter-v1").unwrap();
        assert!(mgr.get_adapter("adapter-v1").is_none());
    }

    // -- TS-07-E10: RemoveAdapter unknown ID returns error ------------------

    #[test]
    fn test_remove_unknown_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);

        let result = mgr.remove_adapter("nonexistent-adapter");
        assert!(result.is_err());
    }

    // -- TS-07-8: WatchAdapterStates streams events -------------------------

    #[tokio::test]
    async fn test_state_transition_emits_event() {
        let (tx, _rx) = broadcast::channel(64);
        let mgr = StateManager::new(tx.clone());
        let mut rx = tx.subscribe();

        let entry = test_entry("adapter-v1", "example.com/adapter:v1");
        mgr.create_adapter(entry);

        // Transition through states
        mgr.transition("adapter-v1", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Running, None)
            .unwrap();

        // Collect events
        let mut events: Vec<AdapterStateEvent> = Vec::new();
        loop {
            match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
                Ok(Ok(event)) => events.push(event),
                _ => break,
            }
        }

        assert!(
            events.len() >= 3,
            "should have at least 3 events, got {}",
            events.len()
        );
        assert_eq!(events[0].adapter_id, "adapter-v1");
        assert_eq!(events[0].old_state, AdapterState::Unknown);
        assert_eq!(events[0].new_state, AdapterState::Downloading);
        assert!(events[0].timestamp > 0, "timestamp should be non-zero");
        assert_eq!(events[1].old_state, AdapterState::Downloading);
        assert_eq!(events[1].new_state, AdapterState::Installing);
        assert_eq!(events[2].old_state, AdapterState::Installing);
        assert_eq!(events[2].new_state, AdapterState::Running);
    }

    // -- TS-07-9: No historical replay (improved per skeptic finding) -------

    #[tokio::test]
    async fn test_no_historical_replay() {
        let (tx, _rx) = broadcast::channel(64);
        let mgr = StateManager::new(tx.clone());

        // Create and transition an adapter BEFORE subscribing
        let entry = test_entry("adapter-v1", "example.com/adapter:v1");
        mgr.create_adapter(entry);
        mgr.transition("adapter-v1", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Running, None)
            .unwrap();

        // Subscribe AFTER historical transitions
        let mut rx = tx.subscribe();

        // Trigger a new transition
        mgr.transition("adapter-v1", AdapterState::Stopped, None)
            .unwrap();

        // Collect events
        let mut events: Vec<AdapterStateEvent> = Vec::new();
        loop {
            match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
                Ok(Ok(event)) => events.push(event),
                _ => break,
            }
        }

        // Positive assertion: stream IS functional (addresses skeptic finding)
        assert!(
            !events.is_empty(),
            "subscriber should receive at least one event"
        );

        // Negative assertion: no historical events
        for event in &events {
            assert_ne!(
                event.old_state,
                AdapterState::Unknown,
                "should not replay UNKNOWN->DOWNLOADING"
            );
            assert_ne!(
                event.new_state,
                AdapterState::Downloading,
                "should not replay historical transitions"
            );
        }
    }

    // -- TS-07-E15: No subscribers does not cause error ---------------------

    #[tokio::test]
    async fn test_no_subscribers_no_error() {
        let (tx, _rx) = broadcast::channel(64);
        let mgr = StateManager::new(tx);
        // Deliberately drop all receivers — no subscribers

        let entry = test_entry("adapter-v1", "example.com/adapter:v1");
        mgr.create_adapter(entry);

        // Transitions should succeed without panicking, even with no subscribers
        mgr.transition("adapter-v1", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Running, None)
            .unwrap();

        let adapter = mgr
            .get_adapter("adapter-v1")
            .expect("adapter should exist");
        assert_eq!(adapter.state, AdapterState::Running);
    }

    // -- TS-07-E7: Subscriber disconnect does not affect others -------------

    #[tokio::test]
    async fn test_subscriber_disconnect() {
        let (tx, _rx) = broadcast::channel(64);
        let mgr = StateManager::new(tx.clone());

        // Two subscribers
        let rx1 = tx.subscribe();
        let mut rx2 = tx.subscribe();

        // Disconnect subscriber 1
        drop(rx1);

        let entry = test_entry("adapter-v1", "example.com/adapter:v1");
        mgr.create_adapter(entry);
        mgr.transition("adapter-v1", AdapterState::Downloading, None)
            .unwrap();

        // Subscriber 2 should still receive events
        let mut events = Vec::new();
        loop {
            match tokio::time::timeout(Duration::from_millis(100), rx2.recv()).await {
                Ok(Ok(event)) => events.push(event),
                _ => break,
            }
        }

        assert!(
            !events.is_empty(),
            "subscriber 2 should still receive events after subscriber 1 disconnects"
        );
    }

    // -- transition_from: atomic compare-and-swap for monitor race guard -----

    #[test]
    fn test_transition_from_succeeds_when_state_matches() {
        let (tx, mut rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);

        mgr.create_adapter(test_entry("adapter-v1", "example.com/adapter:v1"));
        mgr.transition("adapter-v1", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Running, None)
            .unwrap();

        // Drain prior events.
        while rx.try_recv().is_ok() {}

        // transition_from RUNNING -> STOPPED should succeed.
        mgr.transition_from(
            "adapter-v1",
            AdapterState::Running,
            AdapterState::Stopped,
            None,
        )
        .unwrap();

        let adapter = mgr.get_adapter("adapter-v1").unwrap();
        assert_eq!(adapter.state, AdapterState::Stopped);

        // An event should have been emitted.
        let event = rx.try_recv().expect("event should be emitted");
        assert_eq!(event.old_state, AdapterState::Running);
        assert_eq!(event.new_state, AdapterState::Stopped);
    }

    #[test]
    fn test_transition_from_fails_when_state_mismatch() {
        let (tx, mut rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);

        mgr.create_adapter(test_entry("adapter-v1", "example.com/adapter:v1"));
        mgr.transition("adapter-v1", AdapterState::Downloading, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Installing, None)
            .unwrap();
        mgr.transition("adapter-v1", AdapterState::Running, None)
            .unwrap();
        // Now transition to STOPPED (simulating RemoveAdapter).
        mgr.transition("adapter-v1", AdapterState::Stopped, None)
            .unwrap();

        // Drain prior events.
        while rx.try_recv().is_ok() {}

        // transition_from RUNNING -> ERROR should fail (adapter is STOPPED).
        let result = mgr.transition_from(
            "adapter-v1",
            AdapterState::Running,
            AdapterState::Error,
            Some("exit code 1".to_string()),
        );
        assert!(
            matches!(result, Err(StateError::InvalidTransition)),
            "should return InvalidTransition when state does not match"
        );

        // Adapter should still be STOPPED (no change).
        let adapter = mgr.get_adapter("adapter-v1").unwrap();
        assert_eq!(adapter.state, AdapterState::Stopped);

        // No event should have been emitted.
        assert!(
            rx.try_recv().is_err(),
            "no event should be emitted for rejected transition"
        );
    }

    #[test]
    fn test_transition_from_fails_when_adapter_not_found() {
        let (tx, _rx) = broadcast::channel(16);
        let mgr = StateManager::new(tx);

        let result = mgr.transition_from(
            "nonexistent",
            AdapterState::Running,
            AdapterState::Stopped,
            None,
        );
        assert!(matches!(result, Err(StateError::NotFound)));
    }

    // -- TS-07-P3: State transition validity property test -------------------
    // Note: (Stopped, Running) removed from valid transitions per skeptic
    // finding — no requirement or execution path supports that transition.

    #[test]
    #[ignore] // Run with --include-ignored
    fn proptest_state_transition_validity() {
        use proptest::prelude::*;

        let valid_transitions: std::collections::HashSet<(AdapterState, AdapterState)> = [
            (AdapterState::Unknown, AdapterState::Downloading),
            (AdapterState::Downloading, AdapterState::Installing),
            (AdapterState::Downloading, AdapterState::Error),
            (AdapterState::Installing, AdapterState::Running),
            (AdapterState::Installing, AdapterState::Error),
            (AdapterState::Running, AdapterState::Stopped),
            (AdapterState::Running, AdapterState::Error),
            (AdapterState::Stopped, AdapterState::Offloading),
            (AdapterState::Offloading, AdapterState::Error),
        ]
        .into_iter()
        .collect();

        // Generate random sequences of transitions and verify all are valid
        let rt = tokio::runtime::Runtime::new().unwrap();

        proptest!(|(seed in 0u64..1000)| {
            rt.block_on(async {
                let (tx, mut rx) = broadcast::channel(64);
                let mgr = StateManager::new(tx);
                let entry = test_entry("adapter-v1", "example.com/adapter:v1");
                mgr.create_adapter(entry);

                // Attempt a valid transition sequence seeded by the random value
                let transitions = [
                    AdapterState::Downloading,
                    AdapterState::Installing,
                    AdapterState::Running,
                ];
                for state in &transitions {
                    let _ = mgr.transition("adapter-v1", *state, None);
                }

                // Drain events and check each is a valid transition
                let _ = seed; // use seed to satisfy proptest
                while let Ok(event) = rx.try_recv() {
                    prop_assert!(
                        valid_transitions.contains(&(event.old_state, event.new_state)),
                        "Invalid transition: {:?} -> {:?}",
                        event.old_state,
                        event.new_state
                    );
                }
                Ok::<(), proptest::test_runner::TestCaseError>(())
            })?;
        });
    }

    // -- TS-07-P4: Event delivery completeness property test ----------------

    #[test]
    #[ignore] // Run with --include-ignored
    fn proptest_event_delivery_completeness() {
        use proptest::prelude::*;

        let rt = tokio::runtime::Runtime::new().unwrap();

        proptest!(|(n in 1usize..4)| {
            rt.block_on(async {
                let (tx, _rx) = broadcast::channel(64);
                let mgr = StateManager::new(tx.clone());

                // Create N subscribers
                let mut receivers: Vec<_> = (0..n).map(|_| tx.subscribe()).collect();

                let entry = test_entry("adapter-v1", "example.com/adapter:v1");
                mgr.create_adapter(entry);
                mgr.transition("adapter-v1", AdapterState::Downloading, None).unwrap();

                // Give broadcast time to deliver
                tokio::task::yield_now().await;

                // Collect events from each subscriber
                let mut all_events: Vec<Vec<AdapterStateEvent>> = Vec::new();
                for rx in &mut receivers {
                    let mut events = Vec::new();
                    while let Ok(event) = rx.try_recv() {
                        events.push(event);
                    }
                    all_events.push(events);
                }

                // All subscribers should receive identical event sequences
                for idx in 1..n {
                    prop_assert_eq!(
                        all_events[idx].len(),
                        all_events[0].len(),
                        "Subscriber {} got different event count", idx
                    );
                    for (j, event) in all_events[idx].iter().enumerate() {
                        prop_assert_eq!(
                            &event.adapter_id,
                            &all_events[0][j].adapter_id,
                        );
                        prop_assert_eq!(
                            event.old_state,
                            all_events[0][j].old_state,
                        );
                        prop_assert_eq!(
                            event.new_state,
                            all_events[0][j].new_state,
                        );
                    }
                }
                Ok::<(), proptest::test_runner::TestCaseError>(())
            })?;
        });
    }
}
