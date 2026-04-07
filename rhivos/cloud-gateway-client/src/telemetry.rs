/// Telemetry state aggregation module.
///
/// Maintains current telemetry state and produces aggregated JSON payloads
/// on signal updates, omitting fields that have never been set.
use crate::models::SignalUpdate;

/// Maintains the latest values of subscribed telemetry signals.
#[allow(dead_code)]
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
        // Stub: always returns None regardless of input
        let _ = signal;
        None
    }
}
