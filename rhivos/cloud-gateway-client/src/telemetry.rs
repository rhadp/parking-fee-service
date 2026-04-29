use crate::models::SignalUpdate;

/// Maintains the current telemetry state and produces aggregated JSON payloads.
///
/// Fields that have never been set are omitted from the output.
pub struct TelemetryState {
    _vin: String,
}

impl TelemetryState {
    /// Create a new telemetry state for the given VIN.
    pub fn new(_vin: String) -> Self {
        todo!()
    }

    /// Update the state with a new signal value.
    ///
    /// Returns `Some(json)` with the aggregated telemetry payload if the
    /// state changed, or `None` if the value is a duplicate.
    pub fn update(&mut self, _signal: SignalUpdate) -> Option<String> {
        todo!()
    }
}
