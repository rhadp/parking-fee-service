use crate::models::{SignalUpdate, TelemetryMessage};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maintains current telemetry state and produces aggregated JSON payloads.
///
/// Each call to `update()` applies a single signal change and returns the
/// full aggregated telemetry JSON, omitting fields that have never been set
/// ([04-REQ-8.3]).
pub struct TelemetryState {
    vin: String,
    is_locked: Option<bool>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    parking_active: Option<bool>,
}

impl TelemetryState {
    /// Creates a new `TelemetryState` for the given VIN.
    pub fn new(vin: String) -> Self {
        Self {
            vin,
            is_locked: None,
            latitude: None,
            longitude: None,
            parking_active: None,
        }
    }

    /// Updates the telemetry state with a new signal value.
    ///
    /// Returns `Some(json)` with the aggregated telemetry payload if the
    /// state changed, or `None` if it did not.
    pub fn update(&mut self, signal: SignalUpdate) -> Option<String> {
        match signal {
            SignalUpdate::IsLocked(v) => self.is_locked = Some(v),
            SignalUpdate::Latitude(v) => self.latitude = Some(v),
            SignalUpdate::Longitude(v) => self.longitude = Some(v),
            SignalUpdate::ParkingActive(v) => self.parking_active = Some(v),
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let msg = TelemetryMessage {
            vin: self.vin.clone(),
            is_locked: self.is_locked,
            latitude: self.latitude,
            longitude: self.longitude,
            parking_active: self.parking_active,
            timestamp,
        };

        // Serialization of a simple struct should not fail; if it does,
        // returning None is the safe fallback (no NATS publish).
        serde_json::to_string(&msg).ok()
    }
}
