/// Generated proto code for kuksa.val.v1.
#[allow(clippy::enum_variant_names)]
mod kuksav1 {
    tonic::include_proto!("kuksa.val.v1");
}

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
/// Connects to the broker at `broker_addr`, issues a `Set` request for the
/// given VSS `path` and `value`, then returns. This is the shared helper used
/// by all three sensor binaries (09-REQ-10.2).
///
/// # Errors
/// Returns an error if the broker is unreachable or the Set RPC fails.
pub async fn publish_datapoint(
    broker_addr: &str,
    path: &str,
    value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    use kuksav1::datapoint::Value as KuksaValue;
    use kuksav1::val_client::ValClient;
    use kuksav1::{DataEntry, Datapoint, EntryUpdate, Field, SetRequest};

    let mut client = ValClient::connect(broker_addr.to_string()).await?;

    let kv = match value {
        DatapointValue::Double(d) => KuksaValue::DoubleValue(d),
        DatapointValue::Float(f) => KuksaValue::FloatValue(f),
        DatapointValue::Bool(b) => KuksaValue::BoolValue(b),
    };

    let request = SetRequest {
        updates: vec![EntryUpdate {
            entry: Some(DataEntry {
                path: path.to_string(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(kv),
                }),
                actuator_target: None,
                metadata: None,
            }),
            fields: vec![Field::Value as i32],
        }],
    };

    client.set(request).await?;
    Ok(())
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

    /// Verifies the crate compiles successfully (01-REQ-8.1, TS-01-26).
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
