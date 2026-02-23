//! Adapter lifecycle state machine and manager.
//!
//! Implements the adapter state machine per 04-REQ-7.1 / 04-REQ-7.2,
//! and the `AdapterManager` that tracks all adapters and emits state
//! change events via a tokio broadcast channel.

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;

/// Adapter lifecycle states matching the proto enum values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdapterState {
    Unknown,
    Downloading,
    Installing,
    Running,
    Stopped,
    Error,
    Offloading,
}

impl fmt::Display for AdapterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterState::Unknown => write!(f, "UNKNOWN"),
            AdapterState::Downloading => write!(f, "DOWNLOADING"),
            AdapterState::Installing => write!(f, "INSTALLING"),
            AdapterState::Running => write!(f, "RUNNING"),
            AdapterState::Stopped => write!(f, "STOPPED"),
            AdapterState::Error => write!(f, "ERROR"),
            AdapterState::Offloading => write!(f, "OFFLOADING"),
        }
    }
}

impl AdapterState {
    /// Convert from the proto enum integer value to AdapterState.
    pub fn from_proto(value: i32) -> Self {
        match value {
            1 => AdapterState::Downloading,
            2 => AdapterState::Installing,
            3 => AdapterState::Running,
            4 => AdapterState::Stopped,
            5 => AdapterState::Error,
            6 => AdapterState::Offloading,
            _ => AdapterState::Unknown,
        }
    }

    /// Convert to the proto enum integer value.
    pub fn to_proto(self) -> i32 {
        match self {
            AdapterState::Unknown => 0,
            AdapterState::Downloading => 1,
            AdapterState::Installing => 2,
            AdapterState::Running => 3,
            AdapterState::Stopped => 4,
            AdapterState::Error => 5,
            AdapterState::Offloading => 6,
        }
    }
}

/// Error returned when an invalid state transition is attempted.
#[derive(Debug, Clone, PartialEq)]
pub struct InvalidTransition {
    pub from: AdapterState,
    pub to: AdapterState,
}

impl fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid state transition: {} -> {}",
            self.from, self.to
        )
    }
}

impl std::error::Error for InvalidTransition {}

/// Check whether a state transition is valid per 04-REQ-7.1.
///
/// Valid transitions:
/// - UNKNOWN -> DOWNLOADING
/// - DOWNLOADING -> INSTALLING
/// - DOWNLOADING -> ERROR
/// - INSTALLING -> RUNNING
/// - INSTALLING -> ERROR
/// - RUNNING -> STOPPED
/// - STOPPED -> OFFLOADING
/// - STOPPED -> DOWNLOADING (re-install)
/// - OFFLOADING -> UNKNOWN (removed)
/// - ERROR -> DOWNLOADING (retry)
pub fn is_valid_transition(from: AdapterState, to: AdapterState) -> bool {
    matches!(
        (from, to),
        (AdapterState::Unknown, AdapterState::Downloading)
            | (AdapterState::Downloading, AdapterState::Installing)
            | (AdapterState::Downloading, AdapterState::Error)
            | (AdapterState::Installing, AdapterState::Running)
            | (AdapterState::Installing, AdapterState::Error)
            | (AdapterState::Running, AdapterState::Stopped)
            | (AdapterState::Stopped, AdapterState::Offloading)
            | (AdapterState::Stopped, AdapterState::Downloading)
            | (AdapterState::Offloading, AdapterState::Unknown)
            | (AdapterState::Error, AdapterState::Downloading)
    )
}

/// Attempt a state transition. Returns Ok(new_state) if valid,
/// Err(InvalidTransition) if not.
pub fn try_transition(
    from: AdapterState,
    to: AdapterState,
) -> Result<AdapterState, InvalidTransition> {
    if is_valid_transition(from, to) {
        Ok(to)
    } else {
        Err(InvalidTransition { from, to })
    }
}

/// Event emitted when an adapter changes state.
#[derive(Debug, Clone)]
pub struct StateEvent {
    pub adapter_id: String,
    pub old_state: AdapterState,
    pub new_state: AdapterState,
    pub timestamp: i64,
}

/// Record for a single adapter managed by the AdapterManager.
#[derive(Debug, Clone)]
pub struct AdapterRecord {
    pub adapter_id: String,
    pub image_ref: String,
    pub checksum_sha256: String,
    pub state: AdapterState,
    pub last_active: Instant,
    pub container_id: Option<String>,
}

/// Manages a map of adapters and enforces state machine transitions.
///
/// Broadcasts `StateEvent` on every transition so that
/// `WatchAdapterStates` streams can relay them to clients.
pub struct AdapterManager {
    adapters: HashMap<String, AdapterRecord>,
    state_tx: broadcast::Sender<StateEvent>,
    pub offload_timeout: Duration,
}

impl AdapterManager {
    /// Create a new AdapterManager with the given offload timeout.
    pub fn new(offload_timeout: Duration) -> Self {
        let (state_tx, _) = broadcast::channel(256);
        AdapterManager {
            adapters: HashMap::new(),
            state_tx,
            offload_timeout,
        }
    }

    /// Subscribe to state change events.
    pub fn subscribe(&self) -> broadcast::Receiver<StateEvent> {
        self.state_tx.subscribe()
    }

    /// Register a new adapter in UNKNOWN state and immediately transition
    /// to DOWNLOADING. Returns the adapter_id.
    ///
    /// Returns Err if an adapter with the same image_ref is already registered
    /// and not in UNKNOWN state (04-REQ-4.E1).
    pub fn install_adapter(
        &mut self,
        adapter_id: String,
        image_ref: String,
        checksum_sha256: String,
    ) -> Result<(), String> {
        // Check if already installed (by image_ref) — 04-REQ-4.E1
        for record in self.adapters.values() {
            if record.image_ref == image_ref && record.state != AdapterState::Unknown {
                return Err(format!(
                    "adapter with image_ref {} is already installed (state: {})",
                    image_ref, record.state
                ));
            }
        }

        let record = AdapterRecord {
            adapter_id: adapter_id.clone(),
            image_ref,
            checksum_sha256,
            state: AdapterState::Unknown,
            last_active: Instant::now(),
            container_id: None,
        };
        self.adapters.insert(adapter_id.clone(), record);

        // Transition UNKNOWN -> DOWNLOADING
        self.transition(&adapter_id, AdapterState::Downloading)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Attempt to transition an adapter to a new state.
    ///
    /// Validates the transition per 04-REQ-7.1, emits a StateEvent
    /// on success, and logs a warning on rejection (04-REQ-7.2).
    pub fn transition(
        &mut self,
        adapter_id: &str,
        new_state: AdapterState,
    ) -> Result<AdapterState, InvalidTransition> {
        let record = match self.adapters.get_mut(adapter_id) {
            Some(r) => r,
            None => {
                return Err(InvalidTransition {
                    from: AdapterState::Unknown,
                    to: new_state,
                });
            }
        };

        let old_state = record.state;
        try_transition(old_state, new_state)?;

        record.state = new_state;
        record.last_active = Instant::now();

        let event = StateEvent {
            adapter_id: adapter_id.to_string(),
            old_state,
            new_state,
            timestamp: now_unix_timestamp(),
        };

        // Best-effort send; receivers may have been dropped
        let _ = self.state_tx.send(event);

        Ok(new_state)
    }

    /// Remove an adapter from the known adapters map.
    ///
    /// Returns Err if the adapter_id is not found (04-REQ-4.E2).
    pub fn remove_adapter(&mut self, adapter_id: &str) -> Result<AdapterRecord, String> {
        self.adapters
            .remove(adapter_id)
            .ok_or_else(|| format!("adapter {} not found", adapter_id))
    }

    /// List all known adapters.
    pub fn list_adapters(&self) -> Vec<&AdapterRecord> {
        self.adapters.values().collect()
    }

    /// Get a single adapter's record by ID.
    ///
    /// Returns None if the adapter_id is not found.
    pub fn get_adapter(&self, adapter_id: &str) -> Option<&AdapterRecord> {
        self.adapters.get(adapter_id)
    }

    /// Return a list of adapters currently in the STOPPED state.
    pub fn stopped_adapters(&self) -> Vec<&AdapterRecord> {
        self.adapters
            .values()
            .filter(|r| r.state == AdapterState::Stopped)
            .collect()
    }
}

/// Get the current Unix timestamp in seconds.
fn now_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TS-04-27: Valid state transitions enforced
    // Requirement: 04-REQ-7.1
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_state_transitions() {
        let valid_transitions = vec![
            (AdapterState::Unknown, AdapterState::Downloading),
            (AdapterState::Downloading, AdapterState::Installing),
            (AdapterState::Downloading, AdapterState::Error),
            (AdapterState::Installing, AdapterState::Running),
            (AdapterState::Installing, AdapterState::Error),
            (AdapterState::Running, AdapterState::Stopped),
            (AdapterState::Stopped, AdapterState::Offloading),
            (AdapterState::Stopped, AdapterState::Downloading),
            (AdapterState::Offloading, AdapterState::Unknown),
            (AdapterState::Error, AdapterState::Downloading),
        ];

        for (from, to) in valid_transitions {
            assert!(
                is_valid_transition(from, to),
                "expected valid transition: {} -> {}",
                from,
                to,
            );
            assert!(
                try_transition(from, to).is_ok(),
                "try_transition should succeed for {} -> {}",
                from,
                to,
            );
        }
    }

    // -----------------------------------------------------------------------
    // TS-04-28: Invalid state transitions rejected
    // Requirement: 04-REQ-7.2
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_state_transitions() {
        let invalid_transitions = vec![
            (AdapterState::Unknown, AdapterState::Running),
            (AdapterState::Unknown, AdapterState::Installing),
            (AdapterState::Unknown, AdapterState::Stopped),
            (AdapterState::Downloading, AdapterState::Stopped),
            (AdapterState::Downloading, AdapterState::Running),
            (AdapterState::Installing, AdapterState::Downloading),
            (AdapterState::Installing, AdapterState::Stopped),
            (AdapterState::Running, AdapterState::Downloading),
            (AdapterState::Running, AdapterState::Installing),
            (AdapterState::Offloading, AdapterState::Running),
            (AdapterState::Offloading, AdapterState::Stopped),
            (AdapterState::Error, AdapterState::Running),
            (AdapterState::Error, AdapterState::Installing),
        ];

        for (from, to) in invalid_transitions {
            assert!(
                !is_valid_transition(from, to),
                "expected invalid transition: {} -> {}",
                from,
                to,
            );
            assert!(
                try_transition(from, to).is_err(),
                "try_transition should fail for {} -> {}",
                from,
                to,
            );
        }
    }

    // -----------------------------------------------------------------------
    // TS-04-P4: State Machine Integrity (property test)
    // Property: For any (S, T) not in valid transitions, transition is rejected.
    // Validates: 04-REQ-7.1, 04-REQ-7.2
    // -----------------------------------------------------------------------

    #[test]
    fn test_property_state_machine_integrity() {
        use std::collections::HashSet;

        let all_states = vec![
            AdapterState::Unknown,
            AdapterState::Downloading,
            AdapterState::Installing,
            AdapterState::Running,
            AdapterState::Stopped,
            AdapterState::Offloading,
            AdapterState::Error,
        ];

        let valid: HashSet<(AdapterState, AdapterState)> = [
            (AdapterState::Unknown, AdapterState::Downloading),
            (AdapterState::Downloading, AdapterState::Installing),
            (AdapterState::Downloading, AdapterState::Error),
            (AdapterState::Installing, AdapterState::Running),
            (AdapterState::Installing, AdapterState::Error),
            (AdapterState::Running, AdapterState::Stopped),
            (AdapterState::Stopped, AdapterState::Offloading),
            (AdapterState::Stopped, AdapterState::Downloading),
            (AdapterState::Offloading, AdapterState::Unknown),
            (AdapterState::Error, AdapterState::Downloading),
        ]
        .into_iter()
        .collect();

        // Exhaustive check: for every (from, to) pair, transition validity
        // must match whether the pair is in the valid set.
        for from in &all_states {
            for to in &all_states {
                let expected_valid = valid.contains(&(*from, *to));
                let actual_valid = is_valid_transition(*from, *to);
                assert_eq!(
                    actual_valid, expected_valid,
                    "state machine integrity: {} -> {} should be {}",
                    from,
                    to,
                    if expected_valid { "valid" } else { "invalid" },
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // AdapterManager tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_adapter_manager_install_and_list() {
        let mut mgr = AdapterManager::new(Duration::from_secs(3600));
        mgr.install_adapter(
            "adapter-1".to_string(),
            "localhost:5000/test:v1".to_string(),
            "checksum123".to_string(),
        )
        .unwrap();

        let adapters = mgr.list_adapters();
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].adapter_id, "adapter-1");
        assert_eq!(adapters[0].state, AdapterState::Downloading);
    }

    #[test]
    fn test_adapter_manager_duplicate_install_rejected() {
        let mut mgr = AdapterManager::new(Duration::from_secs(3600));
        mgr.install_adapter(
            "adapter-1".to_string(),
            "localhost:5000/test:v1".to_string(),
            "checksum123".to_string(),
        )
        .unwrap();

        let result = mgr.install_adapter(
            "adapter-2".to_string(),
            "localhost:5000/test:v1".to_string(),
            "checksum456".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_adapter_manager_transition() {
        let mut mgr = AdapterManager::new(Duration::from_secs(3600));
        mgr.install_adapter(
            "adapter-1".to_string(),
            "localhost:5000/test:v1".to_string(),
            "checksum123".to_string(),
        )
        .unwrap();

        // DOWNLOADING -> INSTALLING
        let result = mgr.transition("adapter-1", AdapterState::Installing);
        assert!(result.is_ok());
        assert_eq!(
            mgr.get_adapter("adapter-1").unwrap().state,
            AdapterState::Installing
        );
    }

    #[test]
    fn test_adapter_manager_remove() {
        let mut mgr = AdapterManager::new(Duration::from_secs(3600));
        mgr.install_adapter(
            "adapter-1".to_string(),
            "localhost:5000/test:v1".to_string(),
            "checksum123".to_string(),
        )
        .unwrap();

        let result = mgr.remove_adapter("adapter-1");
        assert!(result.is_ok());
        assert!(mgr.list_adapters().is_empty());
    }

    #[test]
    fn test_adapter_manager_remove_unknown() {
        let mut mgr = AdapterManager::new(Duration::from_secs(3600));
        let result = mgr.remove_adapter("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_adapter_manager_get_unknown() {
        let mgr = AdapterManager::new(Duration::from_secs(3600));
        assert!(mgr.get_adapter("nonexistent").is_none());
    }

    #[test]
    fn test_adapter_manager_state_event_emitted() {
        let mut mgr = AdapterManager::new(Duration::from_secs(3600));
        let mut rx = mgr.subscribe();

        mgr.install_adapter(
            "adapter-1".to_string(),
            "localhost:5000/test:v1".to_string(),
            "checksum123".to_string(),
        )
        .unwrap();

        // Should have received one event for UNKNOWN -> DOWNLOADING
        let event = rx.try_recv().unwrap();
        assert_eq!(event.adapter_id, "adapter-1");
        assert_eq!(event.old_state, AdapterState::Unknown);
        assert_eq!(event.new_state, AdapterState::Downloading);
    }
}
