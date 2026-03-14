use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::model::{AdapterInfo, AdapterState, AdapterStateEvent};

const BROADCAST_CAPACITY: usize = 256;

/// Errors from state-manager operations.
#[derive(Debug, Clone, PartialEq)]
pub enum StateError {
    /// The adapter does not exist.
    NotFound(String),
    /// The requested transition is not permitted by the state machine.
    InvalidTransition { from: AdapterState, to: AdapterState },
    /// An adapter with this ID already exists.
    AlreadyExists(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::NotFound(id) => write!(f, "adapter '{}' not found", id),
            StateError::InvalidTransition { from, to } => {
                write!(f, "invalid transition {} → {}", from, to)
            }
            StateError::AlreadyExists(id) => write!(f, "adapter '{}' already exists", id),
        }
    }
}

impl std::error::Error for StateError {}

/// In-memory adapter state manager with event broadcasting.
pub struct StateManager {
    #[allow(dead_code)]
    adapters: Arc<Mutex<HashMap<String, AdapterInfo>>>,
    tx: broadcast::Sender<AdapterStateEvent>,
}

impl StateManager {
    /// Create a new, empty state manager.
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            adapters: Arc::new(Mutex::new(HashMap::new())),
            tx,
        }
    }

    /// Register a new adapter in DOWNLOADING state.
    pub fn create_adapter(
        &self,
        _adapter_id: &str,
        _image_ref: &str,
        _checksum: &str,
    ) -> Result<(), StateError> {
        todo!("implement StateManager::create_adapter")
    }

    /// Transition an existing adapter to `new_state`, broadcasting the event.
    ///
    /// Returns `StateError::InvalidTransition` for transitions not in the
    /// valid state machine.
    pub fn transition(
        &self,
        _adapter_id: &str,
        _new_state: AdapterState,
    ) -> Result<(), StateError> {
        todo!("implement StateManager::transition")
    }

    /// Look up an adapter by ID.
    pub fn get(&self, _adapter_id: &str) -> Option<AdapterInfo> {
        todo!("implement StateManager::get")
    }

    /// Return all known adapters.
    pub fn list(&self) -> Vec<AdapterInfo> {
        todo!("implement StateManager::list")
    }

    /// Remove an adapter from in-memory state.  Does NOT emit an event.
    pub fn remove(&self, _adapter_id: &str) -> Result<(), StateError> {
        todo!("implement StateManager::remove")
    }

    /// Return the adapter ID of the currently RUNNING adapter, if any.
    pub fn get_running_adapter(&self) -> Option<String> {
        todo!("implement StateManager::get_running_adapter")
    }

    /// Return the adapter IDs whose STOPPED time exceeds `timeout_secs` ago.
    pub fn get_stopped_expired(&self, _timeout_secs: u64) -> Vec<String> {
        todo!("implement StateManager::get_stopped_expired")
    }

    /// Subscribe to state-transition events.
    pub fn subscribe(&self) -> broadcast::Receiver<AdapterStateEvent> {
        self.tx.subscribe()
    }

    /// Update the `stopped_at` field of an adapter (test helper).
    #[cfg(test)]
    pub fn set_stopped_at(&self, _adapter_id: &str, _ts: i64) {
        todo!("implement StateManager::set_stopped_at")
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Valid state transitions
// ---------------------------------------------------------------------------

/// Returns `true` if transitioning `from` → `to` is permitted.
pub fn is_valid_transition(_from: &AdapterState, _to: &AdapterState) -> bool {
    todo!("implement is_valid_transition")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::MockContainerRuntime;
    use crate::service::{install_adapter, remove_adapter, InstallResponse};
    use std::sync::Arc;

    fn make_runtime_ok(checksum: &str) -> Arc<MockContainerRuntime> {
        Arc::new(MockContainerRuntime::new().with_digest(checksum))
    }

    // TS-07-1: Install Adapter Happy Path
    #[tokio::test]
    async fn test_install_happy_path() {
        let manager = Arc::new(StateManager::new());
        let runtime = make_runtime_ok("sha256:abc123def456");
        let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
        let checksum = "sha256:abc123def456";

        let resp: InstallResponse = install_adapter(
            Arc::clone(&manager),
            runtime as Arc<dyn crate::container::ContainerRuntime>,
            image_ref,
            checksum,
        )
        .await
        .expect("install should succeed");

        assert!(!resp.job_id.is_empty(), "job_id must be non-empty");
        assert_eq!(
            resp.adapter_id, "parkhaus-munich-v1.0.0",
            "adapter_id must be derived from image_ref"
        );
        assert_eq!(
            resp.initial_state,
            AdapterState::Downloading,
            "initial returned state must be DOWNLOADING"
        );

        // After the async installation pipeline completes, state must be RUNNING
        let info = manager
            .get("parkhaus-munich-v1.0.0")
            .expect("adapter must exist");
        assert_eq!(info.state, AdapterState::Running);
    }

    // TS-07-2: State Transitions During Install
    #[tokio::test]
    async fn test_state_transitions_during_install() {
        let manager = Arc::new(StateManager::new());
        let mut rx = manager.subscribe();
        let runtime = make_runtime_ok("sha256:abc123def456");

        install_adapter(
            Arc::clone(&manager),
            runtime as Arc<dyn crate::container::ContainerRuntime>,
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
            "sha256:abc123def456",
        )
        .await
        .expect("install should succeed");

        // Collect all events that were sent during install
        let mut events: Vec<AdapterStateEvent> = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }

        // Must have at least UNKNOWN→DOWNLOADING, DOWNLOADING→INSTALLING, INSTALLING→RUNNING
        assert!(events.len() >= 3, "expected at least 3 state events");
        assert_eq!(
            events[0].old_state,
            AdapterState::Unknown,
            "first event old_state must be UNKNOWN"
        );
        assert_eq!(events[0].new_state, AdapterState::Downloading);
        assert_eq!(events[1].old_state, AdapterState::Downloading);
        assert_eq!(events[1].new_state, AdapterState::Installing);
        assert_eq!(events[2].old_state, AdapterState::Installing);
        assert_eq!(events[2].new_state, AdapterState::Running);
    }

    // TS-07-3: Checksum Verification
    #[tokio::test]
    async fn test_checksum_verification() {
        let manager = Arc::new(StateManager::new());
        // Runtime returns matching digest
        let runtime = make_runtime_ok("sha256:abc123def456");

        install_adapter(
            Arc::clone(&manager),
            runtime as Arc<dyn crate::container::ContainerRuntime>,
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
            "sha256:abc123def456",
        )
        .await
        .expect("install with matching checksum should succeed");

        let info = manager
            .get("parkhaus-munich-v1.0.0")
            .expect("adapter must exist");
        assert_ne!(
            info.state,
            AdapterState::Error,
            "adapter must not be in ERROR when checksum matches"
        );
    }

    // TS-07-4: Container Started with Host Networking
    //
    // Tests that run() is called with the correct image_ref and adapter_id.
    // The `--network=host` flag is verified in the PodmanRuntime integration
    // test; here we assert the mock recorded a run call.
    #[tokio::test]
    async fn test_container_host_networking() {
        let manager = Arc::new(StateManager::new());
        let runtime = Arc::new(
            MockContainerRuntime::new().with_digest("sha256:abc123def456"),
        );
        let runtime_clone = Arc::clone(&runtime);

        install_adapter(
            Arc::clone(&manager),
            runtime as Arc<dyn crate::container::ContainerRuntime>,
            "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
            "sha256:abc123def456",
        )
        .await
        .expect("install should succeed");

        assert!(
            runtime_clone.was_run_called(),
            "ContainerRuntime::run must be called during install"
        );
    }

    // TS-07-6: Single Adapter Stops Running Before New Install
    #[tokio::test]
    async fn test_single_adapter_stops_running() {
        let manager = Arc::new(StateManager::new());
        let runtime1 = make_runtime_ok("sha256:checksum1");
        let runtime2 = make_runtime_ok("sha256:checksum2");

        // Install first adapter
        install_adapter(
            Arc::clone(&manager),
            Arc::clone(&runtime1) as Arc<dyn crate::container::ContainerRuntime>,
            "registry.io/repo/old-adapter:v1",
            "sha256:checksum1",
        )
        .await
        .expect("first install should succeed");

        assert_eq!(
            manager.get("old-adapter-v1").expect("exists").state,
            AdapterState::Running
        );

        // Install second adapter — must stop the first
        install_adapter(
            Arc::clone(&manager),
            runtime2 as Arc<dyn crate::container::ContainerRuntime>,
            "registry.io/repo/new-adapter:v1",
            "sha256:checksum2",
        )
        .await
        .expect("second install should succeed");

        assert_eq!(
            manager.get("old-adapter-v1").expect("old adapter exists").state,
            AdapterState::Stopped,
            "previously running adapter must be STOPPED"
        );
        assert_eq!(
            manager.get("new-adapter-v1").expect("new adapter exists").state,
            AdapterState::Running,
            "new adapter must be RUNNING"
        );
    }

    // TS-07-7: Previous Adapter Stopped State emitted before new install starts
    #[tokio::test]
    async fn test_previous_adapter_stopped_state() {
        let manager = Arc::new(StateManager::new());
        let runtime1 = make_runtime_ok("sha256:checksum1");

        // Install and get to RUNNING
        install_adapter(
            Arc::clone(&manager),
            Arc::clone(&runtime1) as Arc<dyn crate::container::ContainerRuntime>,
            "registry.io/repo/adapter-a:v1",
            "sha256:checksum1",
        )
        .await
        .expect("first install should succeed");

        let mut rx = manager.subscribe();
        let runtime2 = make_runtime_ok("sha256:checksum2");

        install_adapter(
            Arc::clone(&manager),
            runtime2 as Arc<dyn crate::container::ContainerRuntime>,
            "registry.io/repo/adapter-b:v1",
            "sha256:checksum2",
        )
        .await
        .expect("second install should succeed");

        // Collect events emitted during second install
        let mut events: Vec<AdapterStateEvent> = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }

        // Find the STOPPED event for adapter-a
        let stop_event = events.iter().find(|e| {
            e.adapter_id == "adapter-a-v1" && e.new_state == AdapterState::Stopped
        });
        assert!(
            stop_event.is_some(),
            "adapter-a must transition to STOPPED before new install"
        );
    }

    // TS-07-8: Watch Adapter States Stream
    #[tokio::test]
    async fn test_watch_state_stream() {
        let manager = Arc::new(StateManager::new());
        let mut rx = manager.subscribe();

        let runtime = make_runtime_ok("sha256:digest");
        install_adapter(
            Arc::clone(&manager),
            runtime as Arc<dyn crate::container::ContainerRuntime>,
            "registry.io/repo/test-adapter:v1",
            "sha256:digest",
        )
        .await
        .expect("install should succeed");

        let event = rx.try_recv().expect("at least one event must be received");
        assert!(!event.adapter_id.is_empty());
        assert_ne!(event.new_state, AdapterState::Unknown);
    }

    // TS-07-9: Multiple Watch Subscribers
    #[tokio::test]
    async fn test_multiple_watch_subscribers() {
        let manager = Arc::new(StateManager::new());

        // Create adapter first so we can transition it
        manager
            .create_adapter("adapter-1", "img:v1", "chk1")
            .expect("create should succeed");

        let mut rx1 = manager.subscribe();
        let mut rx2 = manager.subscribe();

        manager
            .transition("adapter-1", AdapterState::Installing)
            .expect("transition should succeed");

        let e1 = rx1.try_recv().expect("subscriber 1 must receive event");
        let e2 = rx2.try_recv().expect("subscriber 2 must receive event");

        assert_eq!(e1.adapter_id, e2.adapter_id, "adapter_id must match");
        assert_eq!(e1.new_state, e2.new_state, "new_state must match");
    }

    // TS-07-10: State Event Fields
    #[tokio::test]
    async fn test_state_event_fields() {
        let manager = Arc::new(StateManager::new());

        manager
            .create_adapter("adapter-1", "img:v1", "chk1")
            .expect("create should succeed");

        let mut rx = manager.subscribe();

        manager
            .transition("adapter-1", AdapterState::Installing)
            .expect("transition should succeed");

        let event = rx.try_recv().expect("event must be received");
        assert_eq!(event.adapter_id, "adapter-1");
        assert_eq!(event.old_state, AdapterState::Downloading);
        assert_eq!(event.new_state, AdapterState::Installing);
        assert!(event.timestamp > 0, "timestamp must be > 0");
    }

    // TS-07-11: List Adapters
    #[test]
    fn test_list_adapters() {
        let manager = StateManager::new();
        manager
            .create_adapter("a1", "img1:v1", "chk1")
            .expect("create a1");
        manager
            .create_adapter("a2", "img2:v1", "chk2")
            .expect("create a2");

        let list = manager.list();
        assert_eq!(list.len(), 2, "list must return 2 adapters");
    }

    // TS-07-12: Get Adapter Status
    #[test]
    fn test_get_adapter_status() {
        let manager = StateManager::new();
        manager
            .create_adapter("a1", "img1:v1", "chk1")
            .expect("create adapter");

        let info = manager.get("a1").expect("adapter must exist");
        assert_eq!(info.state, AdapterState::Downloading);
        assert_eq!(info.image_ref, "img1:v1");
        assert!(info.created_at > 0, "created_at must be > 0");
    }

    // TS-07-13: Remove Adapter
    #[tokio::test]
    async fn test_remove_adapter() {
        let manager = Arc::new(StateManager::new());
        let runtime = Arc::new(MockContainerRuntime::new().with_digest("sha256:digest"));

        install_adapter(
            Arc::clone(&manager),
            Arc::clone(&runtime) as Arc<dyn crate::container::ContainerRuntime>,
            "registry.io/repo/adapter-a:v1",
            "sha256:digest",
        )
        .await
        .expect("install should succeed");

        let runtime_remove = Arc::clone(&runtime);
        remove_adapter(
            Arc::clone(&manager),
            runtime_remove as Arc<dyn crate::container::ContainerRuntime>,
            "adapter-a-v1",
        )
        .await
        .expect("remove should succeed");

        assert!(
            manager.get("adapter-a-v1").is_none(),
            "adapter must be deleted from state"
        );
        assert!(runtime.was_stop_called(), "stop must be called");
        assert!(runtime.was_remove_called(), "remove must be called");
        assert!(runtime.was_remove_image_called(), "remove_image must be called");
    }

    // TS-07-14: Remove Adapter State Transitions
    #[tokio::test]
    async fn test_remove_adapter_transitions() {
        let manager = Arc::new(StateManager::new());
        let runtime = Arc::new(MockContainerRuntime::new().with_digest("sha256:digest"));

        install_adapter(
            Arc::clone(&manager),
            Arc::clone(&runtime) as Arc<dyn crate::container::ContainerRuntime>,
            "registry.io/repo/adapter-a:v1",
            "sha256:digest",
        )
        .await
        .expect("install should succeed");

        let mut rx = manager.subscribe();

        remove_adapter(
            Arc::clone(&manager),
            runtime as Arc<dyn crate::container::ContainerRuntime>,
            "adapter-a-v1",
        )
        .await
        .expect("remove should succeed");

        let mut events: Vec<AdapterStateEvent> = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }

        let has_stopped = events
            .iter()
            .any(|e| e.new_state == AdapterState::Stopped);
        let has_offloading = events
            .iter()
            .any(|e| e.new_state == AdapterState::Offloading);

        assert!(has_stopped, "RUNNING→STOPPED event must be emitted");
        assert!(has_offloading, "STOPPED→OFFLOADING event must be emitted");
    }

    // TS-07-15: Remove Adapter Events Emitted
    #[tokio::test]
    async fn test_remove_adapter_events() {
        let manager = Arc::new(StateManager::new());
        let runtime = Arc::new(MockContainerRuntime::new().with_digest("sha256:digest"));

        install_adapter(
            Arc::clone(&manager),
            Arc::clone(&runtime) as Arc<dyn crate::container::ContainerRuntime>,
            "registry.io/repo/adapter-a:v1",
            "sha256:digest",
        )
        .await
        .expect("install should succeed");

        let mut rx = manager.subscribe();

        remove_adapter(
            Arc::clone(&manager),
            runtime as Arc<dyn crate::container::ContainerRuntime>,
            "adapter-a-v1",
        )
        .await
        .expect("remove should succeed");

        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert!(count >= 2, "at least 2 events must be emitted during removal");
    }

    // TS-07-16: Automatic Offloading
    #[test]
    fn test_automatic_offloading() {
        let manager = StateManager::new();
        manager
            .create_adapter("a1", "img:v1", "chk")
            .expect("create");
        // Manually transition to STOPPED
        manager
            .transition("a1", AdapterState::Installing)
            .expect("→INSTALLING");
        manager
            .transition("a1", AdapterState::Running)
            .expect("→RUNNING");
        manager
            .transition("a1", AdapterState::Stopped)
            .expect("→STOPPED");

        // Backdate the stopped_at timestamp to exceed the 24 h timeout
        let past = crate::model::now_unix() - (25 * 3600);
        manager.set_stopped_at("a1", past);

        let expired = manager.get_stopped_expired(86400);
        assert!(
            expired.contains(&"a1".to_string()),
            "adapter stopped > timeout must appear in expired list"
        );
    }

    // TS-07-18: Offloading Events Emitted
    #[test]
    fn test_offloading_events() {
        let manager = StateManager::new();
        manager
            .create_adapter("a1", "img:v1", "chk")
            .expect("create");
        manager
            .transition("a1", AdapterState::Installing)
            .expect("→INSTALLING");
        manager
            .transition("a1", AdapterState::Running)
            .expect("→RUNNING");
        manager
            .transition("a1", AdapterState::Stopped)
            .expect("→STOPPED");

        let mut rx = manager.subscribe();

        manager
            .transition("a1", AdapterState::Offloading)
            .expect("→OFFLOADING");

        let event = rx.try_recv().expect("OFFLOADING event must be emitted");
        assert_eq!(event.new_state, AdapterState::Offloading);
    }

    // TS-07-E6: Get Unknown Adapter
    #[test]
    fn test_get_unknown_adapter() {
        let manager = StateManager::new();
        assert!(
            manager.get("nonexistent").is_none(),
            "unknown adapter must return None"
        );
    }
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn all_states() -> Vec<AdapterState> {
        vec![
            AdapterState::Unknown,
            AdapterState::Downloading,
            AdapterState::Installing,
            AdapterState::Running,
            AdapterState::Stopped,
            AdapterState::Error,
            AdapterState::Offloading,
        ]
    }

    fn valid_transitions() -> Vec<(AdapterState, AdapterState)> {
        vec![
            (AdapterState::Downloading, AdapterState::Installing),
            (AdapterState::Downloading, AdapterState::Error),
            (AdapterState::Installing, AdapterState::Running),
            (AdapterState::Installing, AdapterState::Error),
            (AdapterState::Running, AdapterState::Stopped),
            (AdapterState::Running, AdapterState::Error),
            (AdapterState::Stopped, AdapterState::Offloading),
            (AdapterState::Stopped, AdapterState::Running),
        ]
    }

    proptest! {
        // TS-07-P1: State Machine Validity
        // Any (from, to) pair: transition() succeeds iff it is in the valid table.
        #[test]
        #[ignore]
        fn proptest_state_machine_validity(
            from_idx in 0usize..7,
            to_idx   in 0usize..7,
        ) {
            let states = all_states();
            let from = states[from_idx].clone();
            let to = states[to_idx].clone();

            let expected_valid = valid_transitions().contains(&(from.clone(), to.clone()));
            let got = is_valid_transition(&from, &to);

            prop_assert_eq!(
                got, expected_valid,
                "is_valid_transition({}, {}) returned {} but expected {}",
                from, to, got, expected_valid
            );
        }

        // TS-07-P4: State Event Broadcasting
        // For any valid transition, all subscribers receive the event.
        #[test]
        #[ignore]
        fn proptest_state_event_broadcasting(
            n_subs in 1usize..=10usize,
        ) {
            let manager = StateManager::new();
            manager.create_adapter("prop-adapter", "img:v1", "chk").unwrap();

            let mut subscribers: Vec<broadcast::Receiver<AdapterStateEvent>> =
                (0..n_subs).map(|_| manager.subscribe()).collect();

            manager.transition("prop-adapter", AdapterState::Installing).unwrap();

            for (i, rx) in subscribers.iter_mut().enumerate() {
                let event = rx.try_recv().expect(&format!("subscriber {} must receive event", i));
                prop_assert_eq!(&event.adapter_id, "prop-adapter");
                prop_assert_eq!(event.old_state, AdapterState::Downloading);
                prop_assert_eq!(event.new_state, AdapterState::Installing);
                prop_assert!(event.timestamp > 0);
            }
        }

        // TS-07-P6: Inactivity Offloading
        // Adapters stopped longer than the timeout appear in the expired list.
        #[test]
        #[ignore]
        fn proptest_inactivity_offloading(
            timeout_secs in 1u64..=86400u64,
            extra_secs in 1u64..=3600u64,
        ) {
            let manager = StateManager::new();
            let adapter_id = "prop-offload";
            manager.create_adapter(adapter_id, "img:v1", "chk").unwrap();
            manager.transition(adapter_id, AdapterState::Installing).unwrap();
            manager.transition(adapter_id, AdapterState::Running).unwrap();
            manager.transition(adapter_id, AdapterState::Stopped).unwrap();

            let now = crate::model::now_unix();

            // Case 1: adapter past timeout — must appear in expired list
            let past = now - (timeout_secs as i64) - (extra_secs as i64);
            manager.set_stopped_at(adapter_id, past);
            let expired = manager.get_stopped_expired(timeout_secs);
            prop_assert!(
                expired.contains(&adapter_id.to_string()),
                "adapter stopped {} secs ago with timeout {} must be expired",
                timeout_secs + extra_secs,
                timeout_secs
            );

            // Case 2: adapter within timeout — must NOT appear in expired list
            let recent = now - (timeout_secs as i64) + (extra_secs as i64).min(timeout_secs as i64 - 1);
            manager.set_stopped_at(adapter_id, recent.max(now - timeout_secs as i64 + 1));
            let not_expired = manager.get_stopped_expired(timeout_secs);
            prop_assert!(
                !not_expired.contains(&adapter_id.to_string()),
                "adapter stopped recently with timeout {} must NOT be expired",
                timeout_secs
            );
        }
    }
}
