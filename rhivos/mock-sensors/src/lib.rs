//! Mock sensor shared library.
//!
//! The `publish_datapoint` function will be implemented in task group 2.

/// The value type to publish to DATA_BROKER.
#[allow(dead_code)]
pub enum DatapointValue {
    /// A 64-bit floating-point value (VSS type: double).
    Double(f64),
    /// A 32-bit floating-point value (VSS type: float).
    Float(f32),
    /// A boolean value (VSS type: bool).
    Bool(bool),
}

/// VSS signal path for vehicle latitude.
pub const PATH_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
/// VSS signal path for vehicle longitude.
pub const PATH_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
/// VSS signal path for vehicle speed.
pub const PATH_SPEED: &str = "Vehicle.Speed";
/// VSS signal path for driver-side door open state.
pub const PATH_DOOR_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
/// Default DATA_BROKER address.
pub const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";

/// Publishes a single VSS signal value to DATA_BROKER via kuksa.val.v1 gRPC Set RPC.
///
/// # Errors
///
/// Returns an error if the DATA_BROKER is unreachable or the RPC fails.
pub fn publish_datapoint(
    _broker_addr: &str,
    _path: &str,
    _value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    todo!("implement in task group 2: connect via tonic, call kuksa.val.v1 Set RPC")
}
