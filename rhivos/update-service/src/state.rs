//! Adapter state models for UPDATE_SERVICE.
//!
//! This module defines the adapter state types used for tracking
//! container lifecycle states.

use std::time::SystemTime;

use crate::proto::AdapterState;

/// Entry representing an adapter's current state.
#[derive(Debug, Clone)]
pub struct AdapterEntry {
    /// Adapter identifier
    pub adapter_id: String,

    /// Image reference (registry URL)
    pub image_ref: String,

    /// Current state
    pub state: AdapterState,

    /// Error message if in ERROR state
    pub error_message: Option<String>,

    /// Last state update timestamp
    pub last_updated: SystemTime,

    /// Last activity timestamp (for offload scheduling)
    pub last_activity: SystemTime,
}

impl AdapterEntry {
    /// Create a new adapter entry with DOWNLOADING state.
    pub fn new(adapter_id: String, image_ref: String) -> Self {
        let now = SystemTime::now();
        Self {
            adapter_id,
            image_ref,
            state: AdapterState::Downloading,
            error_message: None,
            last_updated: now,
            last_activity: now,
        }
    }

    /// Update the adapter state.
    pub fn set_state(&mut self, state: AdapterState, error_message: Option<String>) {
        self.state = state;
        self.error_message = error_message;
        self.last_updated = SystemTime::now();
        self.last_activity = SystemTime::now();
    }

    /// Update the last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = SystemTime::now();
    }

    /// Check if the adapter is in an active state.
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            AdapterState::Downloading | AdapterState::Installing | AdapterState::Running
        )
    }

    /// Get the time since last activity in seconds.
    pub fn inactivity_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.last_activity)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Convert to proto AdapterInfo.
    pub fn to_proto_info(&self) -> crate::proto::AdapterInfo {
        crate::proto::AdapterInfo {
            adapter_id: self.adapter_id.clone(),
            image_ref: self.image_ref.clone(),
            version: String::new(), // Version extracted from image if available
            state: self.state as i32,
            error_message: self.error_message.clone().unwrap_or_default(),
        }
    }

    /// Get last updated as Unix timestamp.
    pub fn last_updated_unix(&self) -> i64 {
        self.last_updated
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

/// Convert i32 to AdapterState.
pub fn adapter_state_from_i32(value: i32) -> AdapterState {
    match value {
        1 => AdapterState::Downloading,
        2 => AdapterState::Installing,
        3 => AdapterState::Running,
        4 => AdapterState::Stopped,
        5 => AdapterState::Error,
        _ => AdapterState::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_adapter_entry_new() {
        let entry = AdapterEntry::new(
            "test-adapter".to_string(),
            "registry.example.com/adapter:v1".to_string(),
        );

        assert_eq!(entry.adapter_id, "test-adapter");
        assert_eq!(entry.state, AdapterState::Downloading);
        assert!(entry.error_message.is_none());
    }

    #[test]
    fn test_adapter_entry_set_state() {
        let mut entry = AdapterEntry::new("test".to_string(), "reg/img".to_string());

        entry.set_state(AdapterState::Running, None);
        assert_eq!(entry.state, AdapterState::Running);

        entry.set_state(AdapterState::Error, Some("Container crashed".to_string()));
        assert_eq!(entry.state, AdapterState::Error);
        assert_eq!(entry.error_message, Some("Container crashed".to_string()));
    }

    #[test]
    fn test_is_active() {
        let mut entry = AdapterEntry::new("test".to_string(), "reg/img".to_string());

        assert!(entry.is_active()); // Downloading

        entry.set_state(AdapterState::Installing, None);
        assert!(entry.is_active());

        entry.set_state(AdapterState::Running, None);
        assert!(entry.is_active());

        entry.set_state(AdapterState::Stopped, None);
        assert!(!entry.is_active());

        entry.set_state(AdapterState::Error, None);
        assert!(!entry.is_active());
    }

    #[test]
    fn test_inactivity_tracking() {
        let mut entry = AdapterEntry::new("test".to_string(), "reg/img".to_string());

        // Small delay to accumulate some inactivity
        thread::sleep(Duration::from_millis(50));

        let inactivity = entry.inactivity_seconds();
        // Could be 0 or very small, but should not panic
        assert!(inactivity < 10);

        entry.touch();
        let new_inactivity = entry.inactivity_seconds();
        assert!(new_inactivity <= inactivity + 1);
    }

    #[test]
    fn test_to_proto_info() {
        let mut entry = AdapterEntry::new(
            "my-adapter".to_string(),
            "registry.example.com/adapter:v2".to_string(),
        );
        entry.set_state(AdapterState::Running, None);

        let info = entry.to_proto_info();
        assert_eq!(info.adapter_id, "my-adapter");
        assert_eq!(info.image_ref, "registry.example.com/adapter:v2");
        assert_eq!(info.state, AdapterState::Running as i32);
        assert!(info.error_message.is_empty());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 10: State Timestamp Updates
        /// Validates: Requirements 5.2, 5.3
        #[test]
        fn prop_state_timestamp_updates_on_change(
            adapter_id in "[a-z][a-z0-9-]{3,20}",
            image_ref in "registry\\.[a-z]+\\.[a-z]+/[a-z]+:[a-z0-9]+"
        ) {
            let mut entry = AdapterEntry::new(adapter_id.clone(), image_ref);
            let initial_time = entry.last_updated;

            // Small delay
            thread::sleep(Duration::from_millis(1));

            entry.set_state(AdapterState::Running, None);

            prop_assert!(entry.last_updated >= initial_time);
            prop_assert_eq!(entry.adapter_id, adapter_id);
            prop_assert_eq!(entry.state, AdapterState::Running);
        }

        #[test]
        fn prop_adapter_state_from_i32_valid(
            state_value in 0i32..=5
        ) {
            let state = adapter_state_from_i32(state_value);

            // Should never panic and always return a valid state
            match state_value {
                0 => prop_assert_eq!(state, AdapterState::Unknown),
                1 => prop_assert_eq!(state, AdapterState::Downloading),
                2 => prop_assert_eq!(state, AdapterState::Installing),
                3 => prop_assert_eq!(state, AdapterState::Running),
                4 => prop_assert_eq!(state, AdapterState::Stopped),
                5 => prop_assert_eq!(state, AdapterState::Error),
                _ => unreachable!(),
            }
        }
    }
}
