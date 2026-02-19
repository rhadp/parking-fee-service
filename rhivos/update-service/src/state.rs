//! Adapter state machine and persistence.
//!
//! This module defines the adapter state machine ([`AdapterState`]),
//! the persisted adapter entry ([`AdapterEntry`]), and the adapter store
//! ([`AdapterStore`]) that manages state transitions and persistence.
//!
//! # State Machine
//!
//! Valid state transitions (see 04-REQ-3.4):
//!
//! ```text
//! Unknown  ──► Installing ──► Running
//!                  │              │
//!                  ▼              ├──► Stopped ──► Installing
//!                Error ◄─────────┤
//!                  │              └──► Offloading ──► Unknown
//!                  └──► Installing
//! ```
//!
//! # Persistence
//!
//! Adapter entries are serialized to `{data_dir}/adapters.json` as a JSON
//! array. The file is loaded on startup; if missing, an empty list is used.
//!
//! # Requirements
//!
//! - 04-REQ-3.4: State machine enforces valid transitions.
//! - 04-REQ-3.5: Persist adapter states to JSON file.
//! - 04-REQ-3.6: Load persisted state on startup.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from the adapter store.
#[derive(Debug, Error)]
pub enum StateError {
    /// An invalid state transition was attempted.
    #[error("invalid transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    /// An adapter was not found by id.
    #[error("adapter '{0}' not found")]
    NotFound(String),

    /// Persistence I/O error.
    #[error("persistence error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StateError>;

// ---------------------------------------------------------------------------
// AdapterState
// ---------------------------------------------------------------------------

/// Lifecycle state of a managed adapter container.
///
/// See the module-level documentation for the valid state transition diagram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "message")]
pub enum AdapterState {
    /// Not installed / removed.
    Unknown,
    /// Container is being created and started.
    Installing,
    /// Container is running.
    Running,
    /// Container has been stopped and removed.
    Stopped,
    /// Container encountered an error (with descriptive message).
    Error(String),
    /// Container is being offloaded (stopped + removed after inactivity).
    Offloading,
}

impl AdapterState {
    /// Check whether a transition from `self` to `target` is valid.
    ///
    /// Valid transitions (per 04-REQ-3.4):
    /// - Unknown → Installing
    /// - Installing → Running
    /// - Installing → Error
    /// - Running → Stopped
    /// - Running → Offloading
    /// - Running → Error
    /// - Offloading → Unknown
    /// - Error → Installing
    /// - Stopped → Installing
    pub fn can_transition_to(&self, target: &AdapterState) -> bool {
        matches!(
            (self, target),
            (AdapterState::Unknown, AdapterState::Installing)
                | (AdapterState::Installing, AdapterState::Running)
                | (AdapterState::Installing, AdapterState::Error(_))
                | (AdapterState::Running, AdapterState::Stopped)
                | (AdapterState::Running, AdapterState::Offloading)
                | (AdapterState::Running, AdapterState::Error(_))
                | (AdapterState::Offloading, AdapterState::Unknown)
                | (AdapterState::Error(_), AdapterState::Installing)
                | (AdapterState::Stopped, AdapterState::Installing)
        )
    }

    /// Convert to the protobuf [`parking_proto::common::AdapterState`] enum.
    pub fn to_proto(&self) -> i32 {
        use parking_proto::common::AdapterState as ProtoState;
        let variant = match self {
            AdapterState::Unknown => ProtoState::Unknown,
            AdapterState::Installing => ProtoState::Installing,
            AdapterState::Running => ProtoState::Running,
            AdapterState::Stopped => ProtoState::Stopped,
            AdapterState::Error(_) => ProtoState::Error,
            AdapterState::Offloading => ProtoState::Offloading,
        };
        variant as i32
    }
}

impl std::fmt::Display for AdapterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterState::Unknown => write!(f, "Unknown"),
            AdapterState::Installing => write!(f, "Installing"),
            AdapterState::Running => write!(f, "Running"),
            AdapterState::Stopped => write!(f, "Stopped"),
            AdapterState::Error(msg) => write!(f, "Error({})", msg),
            AdapterState::Offloading => write!(f, "Offloading"),
        }
    }
}

// ---------------------------------------------------------------------------
// AdapterConfig — env vars passed to the container
// ---------------------------------------------------------------------------

/// Environment variables passed to an adapter container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AdapterConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub databroker_addr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parking_operator_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle_vin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_addr: Option<String>,
}

impl AdapterConfig {
    /// Convert to a `HashMap<String, String>` suitable for passing to podman.
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        if let Some(ref v) = self.databroker_addr {
            env.insert("DATABROKER_ADDR".to_string(), v.clone());
        }
        if let Some(ref v) = self.parking_operator_url {
            env.insert("PARKING_OPERATOR_URL".to_string(), v.clone());
        }
        if let Some(ref v) = self.zone_id {
            env.insert("ZONE_ID".to_string(), v.clone());
        }
        if let Some(ref v) = self.vehicle_vin {
            env.insert("VEHICLE_VIN".to_string(), v.clone());
        }
        if let Some(ref v) = self.listen_addr {
            env.insert("LISTEN_ADDR".to_string(), v.clone());
        }
        env
    }
}

// ---------------------------------------------------------------------------
// AdapterEntry — persisted per-adapter record
// ---------------------------------------------------------------------------

/// A persisted adapter record.
///
/// Contains the adapter's identity, container name, current state,
/// configuration, and timestamps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdapterEntry {
    /// Unique adapter identifier (generated on install).
    pub adapter_id: String,
    /// Container image reference (e.g. `localhost/parking-operator-adaptor:latest`).
    pub image_ref: String,
    /// Image checksum (e.g. `sha256:abc123`).
    pub checksum: String,
    /// Podman container name (e.g. `poa-adapter-001`).
    pub container_name: String,
    /// Current lifecycle state.
    pub state: AdapterState,
    /// Environment variables for the container.
    #[serde(default)]
    pub config: AdapterConfig,
    /// Unix timestamp when the adapter was installed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<i64>,
    /// Unix timestamp when the last session ended (for offloading).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_ended_at: Option<i64>,
}

impl AdapterEntry {
    /// Attempt to transition this adapter to a new state.
    ///
    /// Returns `Ok(old_state)` on success, or [`StateError::InvalidTransition`]
    /// if the transition is not valid.
    pub fn transition(&mut self, new_state: AdapterState) -> Result<AdapterState> {
        if self.state.can_transition_to(&new_state) {
            let old = std::mem::replace(&mut self.state, new_state);
            Ok(old)
        } else {
            Err(StateError::InvalidTransition {
                from: self.state.to_string(),
                to: new_state.to_string(),
            })
        }
    }

    /// Convert to a protobuf [`parking_proto::common::AdapterInfo`].
    pub fn to_proto_info(&self) -> parking_proto::common::AdapterInfo {
        parking_proto::common::AdapterInfo {
            adapter_id: self.adapter_id.clone(),
            name: self.container_name.clone(),
            image_ref: self.image_ref.clone(),
            checksum: self.checksum.clone(),
            version: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// AdapterStore — in-memory store with persistence
// ---------------------------------------------------------------------------

/// In-memory store for adapter entries with JSON file persistence.
///
/// The store loads from `{data_dir}/adapters.json` on construction and
/// saves after every mutation.
#[derive(Debug)]
pub struct AdapterStore {
    /// Map from `adapter_id` to entry.
    entries: HashMap<String, AdapterEntry>,
    /// Path to the persistence file.
    persistence_path: PathBuf,
}

/// Name of the persistence file within the data directory.
const PERSISTENCE_FILE: &str = "adapters.json";

impl AdapterStore {
    /// Create a new store, loading any existing persisted state.
    ///
    /// If the persistence file does not exist, an empty store is created.
    /// If the data directory does not exist, it is created.
    ///
    /// # Requirements
    ///
    /// - 04-REQ-3.5: Persist adapter states to a JSON file.
    /// - 04-REQ-3.6: Load persisted adapter states on startup.
    pub fn load(data_dir: &str) -> Result<Self> {
        let dir = Path::new(data_dir);
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }

        let path = dir.join(PERSISTENCE_FILE);
        let entries = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let list: Vec<AdapterEntry> = serde_json::from_str(&content)?;
            info!(
                count = list.len(),
                path = %path.display(),
                "loaded persisted adapter state"
            );
            list.into_iter()
                .map(|e| (e.adapter_id.clone(), e))
                .collect()
        } else {
            info!(path = %path.display(), "no persisted state found, starting fresh");
            HashMap::new()
        };

        Ok(Self {
            entries,
            persistence_path: path,
        })
    }

    /// Save current state to the persistence file.
    pub fn save(&self) -> Result<()> {
        let list: Vec<&AdapterEntry> = self.entries.values().collect();
        let json = serde_json::to_string_pretty(&list)?;
        std::fs::write(&self.persistence_path, json)?;
        Ok(())
    }

    /// Insert or replace an adapter entry, then persist.
    pub fn insert(&mut self, entry: AdapterEntry) -> Result<()> {
        self.entries.insert(entry.adapter_id.clone(), entry);
        self.save()
    }

    /// Get a reference to an adapter entry by id.
    pub fn get(&self, adapter_id: &str) -> Option<&AdapterEntry> {
        self.entries.get(adapter_id)
    }

    /// Get a mutable reference to an adapter entry by id.
    pub fn get_mut(&mut self, adapter_id: &str) -> Option<&mut AdapterEntry> {
        self.entries.get_mut(adapter_id)
    }

    /// Transition an adapter to a new state, persist, and return the old state.
    ///
    /// # Errors
    ///
    /// - [`StateError::NotFound`] if the adapter does not exist.
    /// - [`StateError::InvalidTransition`] if the transition is invalid.
    pub fn transition(
        &mut self,
        adapter_id: &str,
        new_state: AdapterState,
    ) -> Result<AdapterState> {
        let entry = self
            .entries
            .get_mut(adapter_id)
            .ok_or_else(|| StateError::NotFound(adapter_id.to_string()))?;

        let old_state = entry.transition(new_state)?;
        self.save()?;
        Ok(old_state)
    }

    /// Remove an adapter entry, persist, and return the removed entry.
    pub fn remove(&mut self, adapter_id: &str) -> Result<AdapterEntry> {
        let entry = self
            .entries
            .remove(adapter_id)
            .ok_or_else(|| StateError::NotFound(adapter_id.to_string()))?;
        self.save()?;
        Ok(entry)
    }

    /// List all adapter entries.
    pub fn list(&self) -> Vec<&AdapterEntry> {
        self.entries.values().collect()
    }

    /// Number of adapters in the store.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Check if an adapter with the given image_ref already exists and is running.
    pub fn find_running_by_image(&self, image_ref: &str) -> Option<&AdapterEntry> {
        self.entries
            .values()
            .find(|e| e.image_ref == image_ref && e.state == AdapterState::Running)
    }

    /// Update `session_ended_at` for an adapter and persist.
    pub fn mark_session_ended(&mut self, adapter_id: &str, timestamp: i64) -> Result<()> {
        let entry = self
            .entries
            .get_mut(adapter_id)
            .ok_or_else(|| StateError::NotFound(adapter_id.to_string()))?;
        entry.session_ended_at = Some(timestamp);
        self.save()
    }

    /// Clear `session_ended_at` for an adapter (session started) and persist.
    pub fn clear_session_ended(&mut self, adapter_id: &str) -> Result<()> {
        let entry = self
            .entries
            .get_mut(adapter_id)
            .ok_or_else(|| StateError::NotFound(adapter_id.to_string()))?;
        entry.session_ended_at = None;
        self.save()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- AdapterState tests ----

    #[test]
    fn valid_transition_unknown_to_installing() {
        assert!(AdapterState::Unknown.can_transition_to(&AdapterState::Installing));
    }

    #[test]
    fn valid_transition_installing_to_running() {
        assert!(AdapterState::Installing.can_transition_to(&AdapterState::Running));
    }

    #[test]
    fn valid_transition_installing_to_error() {
        assert!(AdapterState::Installing
            .can_transition_to(&AdapterState::Error("fail".to_string())));
    }

    #[test]
    fn valid_transition_running_to_stopped() {
        assert!(AdapterState::Running.can_transition_to(&AdapterState::Stopped));
    }

    #[test]
    fn valid_transition_running_to_offloading() {
        assert!(AdapterState::Running.can_transition_to(&AdapterState::Offloading));
    }

    #[test]
    fn valid_transition_running_to_error() {
        assert!(AdapterState::Running
            .can_transition_to(&AdapterState::Error("crash".to_string())));
    }

    #[test]
    fn valid_transition_offloading_to_unknown() {
        assert!(AdapterState::Offloading.can_transition_to(&AdapterState::Unknown));
    }

    #[test]
    fn valid_transition_error_to_installing() {
        assert!(AdapterState::Error("prev".to_string()).can_transition_to(&AdapterState::Installing));
    }

    #[test]
    fn valid_transition_stopped_to_installing() {
        assert!(AdapterState::Stopped.can_transition_to(&AdapterState::Installing));
    }

    // ---- Invalid transitions ----

    #[test]
    fn invalid_transition_unknown_to_running() {
        assert!(!AdapterState::Unknown.can_transition_to(&AdapterState::Running));
    }

    #[test]
    fn invalid_transition_unknown_to_stopped() {
        assert!(!AdapterState::Unknown.can_transition_to(&AdapterState::Stopped));
    }

    #[test]
    fn invalid_transition_installing_to_stopped() {
        assert!(!AdapterState::Installing.can_transition_to(&AdapterState::Stopped));
    }

    #[test]
    fn invalid_transition_installing_to_offloading() {
        assert!(!AdapterState::Installing.can_transition_to(&AdapterState::Offloading));
    }

    #[test]
    fn invalid_transition_running_to_installing() {
        assert!(!AdapterState::Running.can_transition_to(&AdapterState::Installing));
    }

    #[test]
    fn invalid_transition_running_to_unknown() {
        assert!(!AdapterState::Running.can_transition_to(&AdapterState::Unknown));
    }

    #[test]
    fn invalid_transition_stopped_to_running() {
        assert!(!AdapterState::Stopped.can_transition_to(&AdapterState::Running));
    }

    #[test]
    fn invalid_transition_stopped_to_offloading() {
        assert!(!AdapterState::Stopped.can_transition_to(&AdapterState::Offloading));
    }

    #[test]
    fn invalid_transition_offloading_to_running() {
        assert!(!AdapterState::Offloading.can_transition_to(&AdapterState::Running));
    }

    #[test]
    fn invalid_transition_error_to_running() {
        assert!(!AdapterState::Error("e".to_string()).can_transition_to(&AdapterState::Running));
    }

    #[test]
    fn invalid_transition_error_to_unknown() {
        assert!(
            !AdapterState::Error("e".to_string()).can_transition_to(&AdapterState::Unknown)
        );
    }

    #[test]
    fn self_transition_not_allowed() {
        assert!(!AdapterState::Unknown.can_transition_to(&AdapterState::Unknown));
        assert!(!AdapterState::Running.can_transition_to(&AdapterState::Running));
        assert!(!AdapterState::Installing.can_transition_to(&AdapterState::Installing));
        assert!(!AdapterState::Stopped.can_transition_to(&AdapterState::Stopped));
        assert!(!AdapterState::Offloading.can_transition_to(&AdapterState::Offloading));
    }

    // ---- AdapterState Display ----

    #[test]
    fn adapter_state_display() {
        assert_eq!(AdapterState::Unknown.to_string(), "Unknown");
        assert_eq!(AdapterState::Installing.to_string(), "Installing");
        assert_eq!(AdapterState::Running.to_string(), "Running");
        assert_eq!(AdapterState::Stopped.to_string(), "Stopped");
        assert_eq!(
            AdapterState::Error("boom".to_string()).to_string(),
            "Error(boom)"
        );
        assert_eq!(AdapterState::Offloading.to_string(), "Offloading");
    }

    // ---- AdapterState to_proto ----

    #[test]
    fn adapter_state_to_proto() {
        use parking_proto::common::AdapterState as ProtoState;

        assert_eq!(AdapterState::Unknown.to_proto(), ProtoState::Unknown as i32);
        assert_eq!(
            AdapterState::Installing.to_proto(),
            ProtoState::Installing as i32
        );
        assert_eq!(
            AdapterState::Running.to_proto(),
            ProtoState::Running as i32
        );
        assert_eq!(
            AdapterState::Stopped.to_proto(),
            ProtoState::Stopped as i32
        );
        assert_eq!(
            AdapterState::Error("e".to_string()).to_proto(),
            ProtoState::Error as i32
        );
        assert_eq!(
            AdapterState::Offloading.to_proto(),
            ProtoState::Offloading as i32
        );
    }

    // ---- AdapterState serde round-trip ----

    #[test]
    fn adapter_state_serde_round_trip() {
        let states = vec![
            AdapterState::Unknown,
            AdapterState::Installing,
            AdapterState::Running,
            AdapterState::Stopped,
            AdapterState::Error("test error".to_string()),
            AdapterState::Offloading,
        ];
        for state in states {
            let json = serde_json::to_string(&state).unwrap();
            let deserialized: AdapterState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, deserialized, "round-trip failed for {}", state);
        }
    }

    // ---- AdapterConfig tests ----

    #[test]
    fn adapter_config_to_env_vars_full() {
        let config = AdapterConfig {
            databroker_addr: Some("http://kuksa:55555".to_string()),
            parking_operator_url: Some("http://operator:8082".to_string()),
            zone_id: Some("zone-1".to_string()),
            vehicle_vin: Some("VIN001".to_string()),
            listen_addr: Some("0.0.0.0:50054".to_string()),
        };
        let env = config.to_env_vars();
        assert_eq!(env.get("DATABROKER_ADDR").unwrap(), "http://kuksa:55555");
        assert_eq!(
            env.get("PARKING_OPERATOR_URL").unwrap(),
            "http://operator:8082"
        );
        assert_eq!(env.get("ZONE_ID").unwrap(), "zone-1");
        assert_eq!(env.get("VEHICLE_VIN").unwrap(), "VIN001");
        assert_eq!(env.get("LISTEN_ADDR").unwrap(), "0.0.0.0:50054");
        assert_eq!(env.len(), 5);
    }

    #[test]
    fn adapter_config_to_env_vars_partial() {
        let config = AdapterConfig {
            databroker_addr: Some("http://kuksa:55555".to_string()),
            ..Default::default()
        };
        let env = config.to_env_vars();
        assert_eq!(env.len(), 1);
        assert_eq!(env.get("DATABROKER_ADDR").unwrap(), "http://kuksa:55555");
    }

    #[test]
    fn adapter_config_to_env_vars_empty() {
        let config = AdapterConfig::default();
        let env = config.to_env_vars();
        assert!(env.is_empty());
    }

    #[test]
    fn adapter_config_serde_round_trip() {
        let config = AdapterConfig {
            databroker_addr: Some("http://kuksa:55555".to_string()),
            parking_operator_url: Some("http://op:8082".to_string()),
            zone_id: Some("zone-1".to_string()),
            vehicle_vin: None,
            listen_addr: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AdapterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    // ---- AdapterEntry tests ----

    fn make_test_entry(id: &str, state: AdapterState) -> AdapterEntry {
        AdapterEntry {
            adapter_id: id.to_string(),
            image_ref: "localhost/test:latest".to_string(),
            checksum: "sha256:abc123".to_string(),
            container_name: format!("poa-{}", id),
            state,
            config: AdapterConfig::default(),
            installed_at: Some(1700000000),
            session_ended_at: None,
        }
    }

    #[test]
    fn entry_transition_valid() {
        let mut entry = make_test_entry("a1", AdapterState::Unknown);
        let old = entry.transition(AdapterState::Installing).unwrap();
        assert_eq!(old, AdapterState::Unknown);
        assert_eq!(entry.state, AdapterState::Installing);
    }

    #[test]
    fn entry_transition_invalid() {
        let mut entry = make_test_entry("a1", AdapterState::Unknown);
        let result = entry.transition(AdapterState::Running);
        assert!(result.is_err());
        // State should remain unchanged
        assert_eq!(entry.state, AdapterState::Unknown);
    }

    #[test]
    fn entry_transition_chain() {
        let mut entry = make_test_entry("a1", AdapterState::Unknown);

        // Unknown → Installing → Running → Stopped → Installing → Running
        entry.transition(AdapterState::Installing).unwrap();
        assert_eq!(entry.state, AdapterState::Installing);

        entry.transition(AdapterState::Running).unwrap();
        assert_eq!(entry.state, AdapterState::Running);

        entry.transition(AdapterState::Stopped).unwrap();
        assert_eq!(entry.state, AdapterState::Stopped);

        entry.transition(AdapterState::Installing).unwrap();
        assert_eq!(entry.state, AdapterState::Installing);

        entry.transition(AdapterState::Running).unwrap();
        assert_eq!(entry.state, AdapterState::Running);
    }

    #[test]
    fn entry_error_recovery_chain() {
        let mut entry = make_test_entry("a1", AdapterState::Unknown);

        // Unknown → Installing → Error → Installing → Running
        entry.transition(AdapterState::Installing).unwrap();
        entry
            .transition(AdapterState::Error("image not found".to_string()))
            .unwrap();
        assert_eq!(
            entry.state,
            AdapterState::Error("image not found".to_string())
        );

        entry.transition(AdapterState::Installing).unwrap();
        entry.transition(AdapterState::Running).unwrap();
        assert_eq!(entry.state, AdapterState::Running);
    }

    #[test]
    fn entry_offloading_chain() {
        let mut entry = make_test_entry("a1", AdapterState::Running);

        // Running → Offloading → Unknown
        entry.transition(AdapterState::Offloading).unwrap();
        assert_eq!(entry.state, AdapterState::Offloading);

        entry.transition(AdapterState::Unknown).unwrap();
        assert_eq!(entry.state, AdapterState::Unknown);
    }

    #[test]
    fn entry_serde_round_trip() {
        let entry = AdapterEntry {
            adapter_id: "adapter-001".to_string(),
            image_ref: "localhost/parking-operator-adaptor:latest".to_string(),
            checksum: "sha256:abc123".to_string(),
            container_name: "poa-adapter-001".to_string(),
            state: AdapterState::Running,
            config: AdapterConfig {
                databroker_addr: Some("http://kuksa:55555".to_string()),
                parking_operator_url: Some("http://op:8082".to_string()),
                zone_id: Some("zone-1".to_string()),
                vehicle_vin: Some("VIN001".to_string()),
                listen_addr: Some("0.0.0.0:50054".to_string()),
            },
            installed_at: Some(1708300800),
            session_ended_at: None,
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        let deserialized: AdapterEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn entry_to_proto_info() {
        let entry = make_test_entry("adapter-001", AdapterState::Running);
        let info = entry.to_proto_info();
        assert_eq!(info.adapter_id, "adapter-001");
        assert_eq!(info.name, "poa-adapter-001");
        assert_eq!(info.image_ref, "localhost/test:latest");
        assert_eq!(info.checksum, "sha256:abc123");
    }

    // ---- AdapterStore tests ----

    #[test]
    fn store_load_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn store_insert_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        let entry = make_test_entry("a1", AdapterState::Running);
        store.insert(entry.clone()).unwrap();

        assert_eq!(store.len(), 1);
        let retrieved = store.get("a1").unwrap();
        assert_eq!(retrieved.adapter_id, "a1");
        assert_eq!(retrieved.state, AdapterState::Running);
    }

    #[test]
    fn store_insert_persists_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        let entry = make_test_entry("a1", AdapterState::Running);
        store.insert(entry).unwrap();

        // Load a new store from the same directory — should see the entry
        let store2 = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(store2.len(), 1);
        assert_eq!(store2.get("a1").unwrap().state, AdapterState::Running);
    }

    #[test]
    fn store_transition_valid() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        store
            .insert(make_test_entry("a1", AdapterState::Unknown))
            .unwrap();

        let old = store
            .transition("a1", AdapterState::Installing)
            .unwrap();
        assert_eq!(old, AdapterState::Unknown);
        assert_eq!(store.get("a1").unwrap().state, AdapterState::Installing);
    }

    #[test]
    fn store_transition_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        store
            .insert(make_test_entry("a1", AdapterState::Unknown))
            .unwrap();

        let result = store.transition("a1", AdapterState::Running);
        assert!(result.is_err());
        // State unchanged
        assert_eq!(store.get("a1").unwrap().state, AdapterState::Unknown);
    }

    #[test]
    fn store_transition_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        let result = store.transition("nonexistent", AdapterState::Installing);
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }

    #[test]
    fn store_remove() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        store
            .insert(make_test_entry("a1", AdapterState::Running))
            .unwrap();
        assert_eq!(store.len(), 1);

        let removed = store.remove("a1").unwrap();
        assert_eq!(removed.adapter_id, "a1");
        assert!(store.is_empty());
    }

    #[test]
    fn store_remove_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        let result = store.remove("nonexistent");
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }

    #[test]
    fn store_list_multiple() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        store
            .insert(make_test_entry("a1", AdapterState::Running))
            .unwrap();
        store
            .insert(make_test_entry("a2", AdapterState::Stopped))
            .unwrap();
        store
            .insert(make_test_entry("a3", AdapterState::Unknown))
            .unwrap();

        let list = store.list();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn store_find_running_by_image() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        let mut entry = make_test_entry("a1", AdapterState::Running);
        entry.image_ref = "my-image:latest".to_string();
        store.insert(entry).unwrap();

        let found = store.find_running_by_image("my-image:latest");
        assert!(found.is_some());
        assert_eq!(found.unwrap().adapter_id, "a1");

        let not_found = store.find_running_by_image("other-image:latest");
        assert!(not_found.is_none());
    }

    #[test]
    fn store_find_running_excludes_non_running() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        let mut entry = make_test_entry("a1", AdapterState::Stopped);
        entry.image_ref = "my-image:latest".to_string();
        store.insert(entry).unwrap();

        let found = store.find_running_by_image("my-image:latest");
        assert!(found.is_none());
    }

    #[test]
    fn store_mark_session_ended() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        store
            .insert(make_test_entry("a1", AdapterState::Running))
            .unwrap();

        store.mark_session_ended("a1", 1700001000).unwrap();
        assert_eq!(store.get("a1").unwrap().session_ended_at, Some(1700001000));
    }

    #[test]
    fn store_clear_session_ended() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        let mut entry = make_test_entry("a1", AdapterState::Running);
        entry.session_ended_at = Some(1700001000);
        store.insert(entry).unwrap();

        store.clear_session_ended("a1").unwrap();
        assert_eq!(store.get("a1").unwrap().session_ended_at, None);
    }

    #[test]
    fn store_persistence_round_trip_multiple_entries() {
        let dir = tempfile::tempdir().unwrap();

        // Create store with entries
        {
            let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();
            store
                .insert(make_test_entry("a1", AdapterState::Running))
                .unwrap();
            store
                .insert(make_test_entry("a2", AdapterState::Stopped))
                .unwrap();

            let mut a3 = make_test_entry("a3", AdapterState::Error("fail".to_string()));
            a3.session_ended_at = Some(1700002000);
            store.insert(a3).unwrap();
        }

        // Load into new store
        let store2 = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(store2.len(), 3);
        assert_eq!(store2.get("a1").unwrap().state, AdapterState::Running);
        assert_eq!(store2.get("a2").unwrap().state, AdapterState::Stopped);
        assert_eq!(
            store2.get("a3").unwrap().state,
            AdapterState::Error("fail".to_string())
        );
        assert_eq!(store2.get("a3").unwrap().session_ended_at, Some(1700002000));
    }

    #[test]
    fn store_transition_persists() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();
            store
                .insert(make_test_entry("a1", AdapterState::Unknown))
                .unwrap();
            store
                .transition("a1", AdapterState::Installing)
                .unwrap();
        }

        // Re-load and check
        let store2 = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(store2.get("a1").unwrap().state, AdapterState::Installing);
    }

    #[test]
    fn store_creates_data_dir_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested").join("subdir");

        let store = AdapterStore::load(nested.to_str().unwrap()).unwrap();
        assert!(store.is_empty());
        assert!(nested.exists());
    }

    // ---- Property-based state machine exhaustive tests ----

    #[test]
    fn all_valid_transitions_accepted() {
        // Exhaustive list of valid edges
        let valid_transitions = vec![
            (AdapterState::Unknown, AdapterState::Installing),
            (AdapterState::Installing, AdapterState::Running),
            (
                AdapterState::Installing,
                AdapterState::Error("e".to_string()),
            ),
            (AdapterState::Running, AdapterState::Stopped),
            (AdapterState::Running, AdapterState::Offloading),
            (AdapterState::Running, AdapterState::Error("e".to_string())),
            (AdapterState::Offloading, AdapterState::Unknown),
            (
                AdapterState::Error("e".to_string()),
                AdapterState::Installing,
            ),
            (AdapterState::Stopped, AdapterState::Installing),
        ];

        for (from, to) in &valid_transitions {
            assert!(
                from.can_transition_to(to),
                "expected valid: {} → {}",
                from,
                to
            );
        }
    }

    #[test]
    fn all_invalid_transitions_rejected() {
        let all_states: Vec<AdapterState> = vec![
            AdapterState::Unknown,
            AdapterState::Installing,
            AdapterState::Running,
            AdapterState::Stopped,
            AdapterState::Error("e".to_string()),
            AdapterState::Offloading,
        ];

        let valid_transitions: Vec<(AdapterState, AdapterState)> = vec![
            (AdapterState::Unknown, AdapterState::Installing),
            (AdapterState::Installing, AdapterState::Running),
            (
                AdapterState::Installing,
                AdapterState::Error("e".to_string()),
            ),
            (AdapterState::Running, AdapterState::Stopped),
            (AdapterState::Running, AdapterState::Offloading),
            (AdapterState::Running, AdapterState::Error("e".to_string())),
            (AdapterState::Offloading, AdapterState::Unknown),
            (
                AdapterState::Error("e".to_string()),
                AdapterState::Installing,
            ),
            (AdapterState::Stopped, AdapterState::Installing),
        ];

        for from in &all_states {
            for to in &all_states {
                let expected_valid = valid_transitions
                    .iter()
                    .any(|(f, t)| {
                        // Compare by variant, not by Error message
                        std::mem::discriminant(f) == std::mem::discriminant(from)
                            && std::mem::discriminant(t) == std::mem::discriminant(to)
                    });

                assert_eq!(
                    from.can_transition_to(to),
                    expected_valid,
                    "transition {} → {} should be {}",
                    from,
                    to,
                    if expected_valid { "valid" } else { "invalid" }
                );
            }
        }
    }

    #[test]
    fn full_lifecycle_transition_sequence() {
        // Test a complete lifecycle:
        // Unknown → Installing → Running → Offloading → Unknown
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        store
            .insert(make_test_entry("lifecycle", AdapterState::Unknown))
            .unwrap();

        store
            .transition("lifecycle", AdapterState::Installing)
            .unwrap();
        store
            .transition("lifecycle", AdapterState::Running)
            .unwrap();
        store
            .transition("lifecycle", AdapterState::Offloading)
            .unwrap();
        store
            .transition("lifecycle", AdapterState::Unknown)
            .unwrap();

        assert_eq!(
            store.get("lifecycle").unwrap().state,
            AdapterState::Unknown
        );
    }

    #[test]
    fn error_recovery_lifecycle() {
        // Unknown → Installing → Error → Installing → Running → Stopped → Installing → Running
        let dir = tempfile::tempdir().unwrap();
        let mut store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();

        store
            .insert(make_test_entry("recovery", AdapterState::Unknown))
            .unwrap();

        store
            .transition("recovery", AdapterState::Installing)
            .unwrap();
        store
            .transition("recovery", AdapterState::Error("image not found".to_string()))
            .unwrap();
        store
            .transition("recovery", AdapterState::Installing)
            .unwrap();
        store
            .transition("recovery", AdapterState::Running)
            .unwrap();
        store
            .transition("recovery", AdapterState::Stopped)
            .unwrap();
        store
            .transition("recovery", AdapterState::Installing)
            .unwrap();
        store
            .transition("recovery", AdapterState::Running)
            .unwrap();

        assert_eq!(
            store.get("recovery").unwrap().state,
            AdapterState::Running
        );
    }

    // ---- StateError tests ----

    #[test]
    fn state_error_display() {
        let err = StateError::InvalidTransition {
            from: "Unknown".to_string(),
            to: "Running".to_string(),
        };
        assert!(err.to_string().contains("Unknown"));
        assert!(err.to_string().contains("Running"));

        let err = StateError::NotFound("test-id".to_string());
        assert!(err.to_string().contains("test-id"));
    }
}
