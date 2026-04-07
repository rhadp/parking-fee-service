//! Telemetry state aggregation module.
//!
//! Maintains current telemetry state and produces aggregated JSON payloads
//! on signal updates, omitting fields that have never been set.
use crate::models::{SignalUpdate, TelemetryMessage};

/// Maintains the latest values of subscribed telemetry signals.
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

    /// Update a signal and return the aggregated JSON if state changed.
    ///
    /// Returns `Some(json_string)` with all known fields, omitting
    /// fields that have never been set. Returns `None` if no state change.
    pub fn update(&mut self, signal: SignalUpdate) -> Option<String> {
        let changed = match signal {
            SignalUpdate::IsLocked(v) => {
                let changed = self.is_locked != Some(v);
                self.is_locked = Some(v);
                changed
            }
            SignalUpdate::Latitude(v) => {
                let changed = self.latitude != Some(v);
                self.latitude = Some(v);
                changed
            }
            SignalUpdate::Longitude(v) => {
                let changed = self.longitude != Some(v);
                self.longitude = Some(v);
                changed
            }
            SignalUpdate::ParkingActive(v) => {
                let changed = self.parking_active != Some(v);
                self.parking_active = Some(v);
                changed
            }
        };

        if !changed {
            return None;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let msg = TelemetryMessage {
            vin: self.vin.clone(),
            is_locked: self.is_locked,
            latitude: self.latitude,
            longitude: self.longitude,
            parking_active: self.parking_active,
            timestamp,
        };

        serde_json::to_string(&msg).ok()
    }
}
