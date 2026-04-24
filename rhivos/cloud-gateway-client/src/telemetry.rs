use crate::models::SignalUpdate;

/// Maintains current telemetry state and produces aggregated JSON payloads.
pub struct TelemetryState {
    _vin: String,
}

impl TelemetryState {
    /// Creates a new `TelemetryState` for the given VIN.
    pub fn new(vin: String) -> Self {
        let _ = vin;
        todo!()
    }

    /// Updates the telemetry state with a new signal value.
    ///
    /// Returns `Some(json)` with the aggregated telemetry payload if the
    /// state changed, or `None` if it did not.
    pub fn update(&mut self, signal: SignalUpdate) -> Option<String> {
        let _ = signal;
        todo!()
    }
}
