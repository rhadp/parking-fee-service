use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{SignalUpdate, TelemetryMessage};

/// Maintains the current telemetry state and produces aggregated JSON payloads.
///
/// Fields that have never been set are omitted from the output.
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
        Self {
            vin,
            is_locked: None,
            latitude: None,
            longitude: None,
            parking_active: None,
        }
    }

    /// Update the state with a new signal value.
    ///
    /// Returns `Some(json)` with the aggregated telemetry payload if the
    /// state changed, or `None` if the value is a duplicate.
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

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System clock is before UNIX epoch")
            .as_secs();

        let msg = TelemetryMessage {
            vin: self.vin.clone(),
            is_locked: self.is_locked,
            latitude: self.latitude,
            longitude: self.longitude,
            parking_active: self.parking_active,
            timestamp,
        };

        Some(serde_json::to_string(&msg).expect("TelemetryMessage serialization should not fail"))
    }
}
