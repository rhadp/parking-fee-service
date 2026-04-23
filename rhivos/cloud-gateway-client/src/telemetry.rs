//! Telemetry state aggregation.
//!
//! Maintains current telemetry state from DATA_BROKER signal updates.
//! Produces aggregated JSON payloads on each change, omitting fields
//! that have never been set.

use crate::models::{SignalUpdate, TelemetryMessage};

/// Maintains current values for all telemetry signals.
///
/// Tracks which signals have been received at least once and produces
/// aggregated JSON payloads on each update.
pub struct TelemetryState {
    vin: String,
    is_locked: Option<bool>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    parking_active: Option<bool>,
}

impl TelemetryState {
    /// Create a new telemetry state for the given VIN.
    pub fn new(vin: String) -> Self {
        TelemetryState {
            vin,
            is_locked: None,
            latitude: None,
            longitude: None,
            parking_active: None,
        }
    }

    /// Apply a signal update and return the aggregated JSON if state changed.
    ///
    /// Returns `Some(json_string)` when the signal value differs from the
    /// previously stored value (or is received for the first time).
    /// Returns `None` if the signal value is identical to the current state,
    /// suppressing spurious telemetry publishes on duplicate updates.
    ///
    /// Fields that have never been set are omitted from the JSON output.
    pub fn update(&mut self, signal: SignalUpdate) -> Option<String> {
        let changed = match &signal {
            SignalUpdate::IsLocked(v) => self.is_locked != Some(*v),
            SignalUpdate::Latitude(v) => self.latitude != Some(*v),
            SignalUpdate::Longitude(v) => self.longitude != Some(*v),
            SignalUpdate::ParkingActive(v) => self.parking_active != Some(*v),
        };

        if !changed {
            return None;
        }

        match signal {
            SignalUpdate::IsLocked(v) => self.is_locked = Some(v),
            SignalUpdate::Latitude(v) => self.latitude = Some(v),
            SignalUpdate::Longitude(v) => self.longitude = Some(v),
            SignalUpdate::ParkingActive(v) => self.parking_active = Some(v),
        }

        let msg = TelemetryMessage {
            vin: self.vin.clone(),
            is_locked: self.is_locked,
            latitude: self.latitude,
            longitude: self.longitude,
            parking_active: self.parking_active,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before UNIX epoch")
                .as_secs(),
        };

        Some(serde_json::to_string(&msg).expect("telemetry serialization failed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-04-7: Telemetry state produces JSON on first update
    #[test]
    fn test_telemetry_first_update() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        let result = state.update(SignalUpdate::IsLocked(true));

        assert!(result.is_some(), "first update should produce JSON");
        let json = result.unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("should be valid JSON");

        assert_eq!(parsed["vin"], "VIN-001");
        assert_eq!(parsed["is_locked"], true);
        assert!(parsed.get("timestamp").is_some(), "should have timestamp");
        assert!(
            parsed.get("latitude").is_none(),
            "unset field should be omitted"
        );
        assert!(
            parsed.get("longitude").is_none(),
            "unset field should be omitted"
        );
        assert!(
            parsed.get("parking_active").is_none(),
            "unset field should be omitted"
        );
    }

    // TS-04-8: Telemetry state omits unset fields
    #[test]
    fn test_telemetry_omits_unset_fields() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        let result = state.update(SignalUpdate::Latitude(48.1351));

        assert!(result.is_some());
        let json = result.unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("should be valid JSON");

        assert_eq!(parsed["latitude"], 48.1351);
        assert!(
            parsed.get("is_locked").is_none(),
            "is_locked should be omitted"
        );
        assert!(
            parsed.get("longitude").is_none(),
            "longitude should be omitted"
        );
        assert!(
            parsed.get("parking_active").is_none(),
            "parking_active should be omitted"
        );
    }

    // Duplicate suppression: repeated identical updates return None.
    // Validates the design contract: "Returns Some(json) if state changed,
    // None if duplicate." Ensures spurious telemetry is not published when
    // DATA_BROKER re-delivers unchanged values (e.g. on reconnect).
    #[test]
    fn test_telemetry_suppresses_duplicate() {
        let mut state = TelemetryState::new("VIN-001".to_string());

        // First update: new value → Some
        let result = state.update(SignalUpdate::IsLocked(true));
        assert!(result.is_some(), "first update should produce JSON");

        // Duplicate update: same value → None
        let result = state.update(SignalUpdate::IsLocked(true));
        assert!(result.is_none(), "duplicate update should return None");

        // Changed update: different value → Some
        let result = state.update(SignalUpdate::IsLocked(false));
        assert!(result.is_some(), "changed value should produce JSON");
        let json = result.unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("should be valid JSON");
        assert_eq!(parsed["is_locked"], false);

        // Duplicate again
        let result = state.update(SignalUpdate::IsLocked(false));
        assert!(result.is_none(), "duplicate after change should return None");

        // Different signal type: new field → Some
        let result = state.update(SignalUpdate::Latitude(48.0));
        assert!(result.is_some(), "new signal type should produce JSON");

        // Duplicate of different signal type
        let result = state.update(SignalUpdate::Latitude(48.0));
        assert!(result.is_none(), "duplicate latitude should return None");

        // Changed latitude
        let result = state.update(SignalUpdate::Latitude(49.0));
        assert!(
            result.is_some(),
            "changed latitude should produce JSON"
        );
    }

    // TS-04-9: Telemetry state includes all known fields
    #[test]
    fn test_telemetry_all_known_fields() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        state.update(SignalUpdate::IsLocked(true));
        state.update(SignalUpdate::Latitude(48.1351));
        state.update(SignalUpdate::Longitude(11.582));
        let result = state.update(SignalUpdate::ParkingActive(true));

        assert!(result.is_some());
        let json = result.unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("should be valid JSON");

        assert_eq!(parsed["is_locked"], true);
        assert_eq!(parsed["latitude"], 48.1351);
        assert_eq!(parsed["longitude"], 11.582);
        assert_eq!(parsed["parking_active"], true);
    }
}
