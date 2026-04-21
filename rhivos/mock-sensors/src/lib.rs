//! Mock sensor shared library.
//!
//! Provides the [`publish_datapoint`] helper used by all three mock sensor
//! binaries to push a single VSS signal value into DATA_BROKER via the
//! kuksa.val.v1 gRPC `Set` RPC (09-REQ-10.2).

// Include the tonic-build generated code for kuksa.val.v1.
pub mod proto {
    pub mod kuksa {
        pub mod val {
            pub mod v1 {
                tonic::include_proto!("kuksa.val.v1");
            }
        }
    }
}

use proto::kuksa::val::v1::{
    val_service_client::ValServiceClient, DataEntry, Datapoint, EntryUpdate, Field, SetRequest,
};

/// VSS signal path for vehicle latitude.
pub const PATH_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
/// VSS signal path for vehicle longitude.
pub const PATH_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
/// VSS signal path for vehicle speed.
pub const PATH_SPEED: &str = "Vehicle.Speed";
/// VSS signal path for driver-side door open state.
pub const PATH_DOOR_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
/// Default DATA_BROKER address used when no flag or env var is provided.
pub const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";

/// The typed value to publish to DATA_BROKER.
pub enum DatapointValue {
    /// A 64-bit floating-point value (VSS type: double).
    Double(f64),
    /// A 32-bit floating-point value (VSS type: float).
    Float(f32),
    /// A boolean value (VSS type: bool).
    Bool(bool),
}

/// Publishes a single VSS signal value to DATA_BROKER via kuksa.val.v1 gRPC
/// `Set` RPC (09-REQ-10.2).
///
/// Connects to `broker_addr`, sets the signal identified by `path` to `value`,
/// and returns.  Returns an error if the connection fails or the RPC returns an
/// error status.
pub async fn publish_datapoint(
    broker_addr: &str,
    path: &str,
    value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ValServiceClient::connect(broker_addr.to_string()).await?;

    let dp_value = match value {
        DatapointValue::Double(v) => proto::kuksa::val::v1::datapoint::Value::Double(v),
        DatapointValue::Float(v) => proto::kuksa::val::v1::datapoint::Value::Float(v),
        DatapointValue::Bool(v) => proto::kuksa::val::v1::datapoint::Value::Bool(v),
    };

    let request = SetRequest {
        updates: vec![EntryUpdate {
            entry: Some(DataEntry {
                path: path.to_string(),
                value: Some(Datapoint {
                    value: Some(dp_value),
                }),
            }),
            fields: vec![Field::Value as i32],
        }],
    };

    let response = client.set(request).await?.into_inner();

    // If the broker reports a top-level error, surface it.
    if let Some(err) = response.error {
        if err.code != 0 {
            return Err(format!(
                "DATA_BROKER Set error {}: {} — {}",
                err.code, err.reason, err.message
            )
            .into());
        }
    }

    // Surface any per-entry errors.
    for entry_err in &response.errors {
        if let Some(ref err) = entry_err.error {
            if err.code != 0 {
                return Err(format!(
                    "DATA_BROKER Set entry error for '{}': {} — {}",
                    entry_err.path, err.reason, err.message
                )
                .into());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vss_path_constants() {
        assert_eq!(PATH_LATITUDE, "Vehicle.CurrentLocation.Latitude");
        assert_eq!(PATH_LONGITUDE, "Vehicle.CurrentLocation.Longitude");
        assert_eq!(PATH_SPEED, "Vehicle.Speed");
        assert_eq!(PATH_DOOR_IS_OPEN, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen");
    }

    #[test]
    fn test_default_broker_addr() {
        assert_eq!(DEFAULT_BROKER_ADDR, "http://localhost:55556");
    }
}
