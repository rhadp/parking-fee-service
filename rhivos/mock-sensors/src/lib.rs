/// VSS signal paths published by mock sensors.
pub const LATITUDE_PATH: &str = "Vehicle.CurrentLocation.Latitude";
pub const LONGITUDE_PATH: &str = "Vehicle.CurrentLocation.Longitude";
pub const SPEED_PATH: &str = "Vehicle.Speed";
pub const DOOR_OPEN_PATH: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

/// Default DATA_BROKER address.
pub const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";

/// Values that can be published to DATA_BROKER.
pub enum DatapointValue {
    Double(f64),
    Float(f32),
    Bool(bool),
}

/// Publish a single VSS signal value to DATA_BROKER via kuksa.val.v1 Set RPC.
///
/// # Errors
/// Returns an error if the broker is unreachable or the Set RPC fails.
pub async fn publish_datapoint(
    _broker_addr: &str,
    _path: &str,
    _value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: implement in task group 2 via kuksa.val.v1 gRPC Set RPC
    Err("publish_datapoint: not implemented".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-09-21: constants are correct VSS paths
    #[test]
    fn test_vss_signal_paths() {
        assert_eq!(LATITUDE_PATH, "Vehicle.CurrentLocation.Latitude");
        assert_eq!(LONGITUDE_PATH, "Vehicle.CurrentLocation.Longitude");
        assert_eq!(SPEED_PATH, "Vehicle.Speed");
        assert_eq!(DOOR_OPEN_PATH, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen");
    }

    // TS-09-21: default broker address is correct
    #[test]
    fn test_default_broker_addr() {
        assert_eq!(DEFAULT_BROKER_ADDR, "http://localhost:55556");
    }
}
