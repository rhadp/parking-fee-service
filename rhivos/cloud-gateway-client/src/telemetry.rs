//! Telemetry types for cloud-gateway-client.
//!
//! This module defines the telemetry message format published to the cloud
//! and internal state tracking for vehicle signals.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Telemetry message published to the cloud.
///
/// Contains all vehicle signal data in a flat structure for CLOUD_GATEWAY.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Telemetry {
    /// ISO8601 timestamp of the telemetry snapshot
    pub timestamp: String,

    /// Vehicle latitude (flat, not nested)
    pub latitude: f64,

    /// Vehicle longitude (flat, not nested)
    pub longitude: f64,

    /// Whether doors are locked
    pub door_locked: bool,

    /// Whether any door is open
    pub door_open: bool,

    /// Whether a parking session is active
    pub parking_session_active: bool,
}

impl Telemetry {
    /// Create a new Telemetry snapshot from the current state.
    pub fn from_state(state: &TelemetryState) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            latitude: state.latitude,
            longitude: state.longitude,
            door_locked: state.door_locked,
            door_open: state.door_open,
            parking_session_active: state.parking_session_active,
        }
    }
}

/// Internal state tracking for vehicle signals.
///
/// Accumulates signal updates from DATA_BROKER before publishing.
#[derive(Debug, Clone, Default)]
pub struct TelemetryState {
    /// Vehicle latitude
    pub latitude: f64,

    /// Vehicle longitude
    pub longitude: f64,

    /// Whether doors are locked
    pub door_locked: bool,

    /// Whether any door is open
    pub door_open: bool,

    /// Whether a parking session is active
    pub parking_session_active: bool,

    /// Timestamp of last update
    pub last_updated: Option<DateTime<Utc>>,
}

impl TelemetryState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the latitude.
    pub fn set_latitude(&mut self, lat: f64) {
        self.latitude = lat;
        self.last_updated = Some(Utc::now());
    }

    /// Update the longitude.
    pub fn set_longitude(&mut self, lng: f64) {
        self.longitude = lng;
        self.last_updated = Some(Utc::now());
    }

    /// Update the door locked state.
    pub fn set_door_locked(&mut self, locked: bool) {
        self.door_locked = locked;
        self.last_updated = Some(Utc::now());
    }

    /// Update the door open state.
    pub fn set_door_open(&mut self, open: bool) {
        self.door_open = open;
        self.last_updated = Some(Utc::now());
    }

    /// Update the parking session active state.
    pub fn set_parking_session_active(&mut self, active: bool) {
        self.parking_session_active = active;
        self.last_updated = Some(Utc::now());
    }

    /// Check if the state has been updated since the given time.
    pub fn has_updates_since(&self, since: DateTime<Utc>) -> bool {
        self.last_updated.is_some_and(|t| t > since)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_serialization() {
        let telem = Telemetry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            latitude: 37.7749,
            longitude: -122.4194,
            door_locked: true,
            door_open: false,
            parking_session_active: true,
        };

        let json = serde_json::to_string(&telem).unwrap();
        assert!(json.contains("\"latitude\":37.7749"));
        assert!(json.contains("\"longitude\":-122.4194"));
        assert!(json.contains("\"door_locked\":true"));
        assert!(json.contains("\"parking_session_active\":true"));

        let parsed: Telemetry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, telem);
    }

    #[test]
    fn test_telemetry_state_updates() {
        let mut state = TelemetryState::new();
        assert!(state.last_updated.is_none());

        state.set_latitude(37.7749);
        assert!(state.last_updated.is_some());
        assert_eq!(state.latitude, 37.7749);
    }

    #[test]
    fn test_telemetry_from_state() {
        let mut state = TelemetryState::new();
        state.set_latitude(37.7749);
        state.set_longitude(-122.4194);
        state.set_door_locked(true);
        state.set_parking_session_active(true);

        let telem = Telemetry::from_state(&state);
        assert_eq!(telem.latitude, 37.7749);
        assert_eq!(telem.longitude, -122.4194);
        assert!(telem.door_locked);
        assert!(telem.parking_session_active);
    }
}
