//! Shared code for mock sensor binaries.
//!
//! This module provides:
//! - Configuration helpers (DATA_BROKER_ADDR resolution)
//! - VSS signal path constants
//! - BrokerWriter: connect to DATA_BROKER and write signal values via kuksa.val.v1 gRPC
//! - Print-usage helper

/// Generated gRPC types for kuksa.val.v1.
pub mod kuksav1 {
    tonic::include_proto!("kuksa.val.v1");
}

/// VSS signal path for vehicle latitude.
pub const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";

/// VSS signal path for vehicle longitude.
pub const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";

/// VSS signal path for vehicle speed.
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";

/// VSS signal path for driver-side front door open state.
pub const SIGNAL_DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

/// Returns the DATA_BROKER address from the `DATA_BROKER_ADDR` environment
/// variable, or the default `http://localhost:55556`.
///
/// Satisfies: 09-REQ-5.1
pub fn get_broker_addr() -> String {
    std::env::var("DATA_BROKER_ADDR").unwrap_or_else(|_| "http://localhost:55556".to_string())
}

/// Error type for broker write operations.
#[derive(Debug)]
pub struct BrokerError(pub String);

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BrokerError {}

/// Write-only gRPC client for the kuksa.val.v1 DATA_BROKER.
///
/// Used by the mock sensor binaries to publish VSS signal values.
pub struct BrokerWriter {
    channel: tonic::transport::Channel,
}

impl BrokerWriter {
    /// Connect to DATA_BROKER at the given address.
    ///
    /// Returns an error if the connection cannot be established.
    /// Satisfies: 09-REQ-1.E2
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let endpoint =
            tonic::transport::Endpoint::from_shared(addr.to_string()).map_err(|e| {
                BrokerError(format!("Invalid DATA_BROKER address '{}': {}", addr, e))
            })?;

        let channel = endpoint.connect().await.map_err(|e| {
            BrokerError(format!(
                "Failed to connect to DATA_BROKER at '{}': {}",
                addr, e
            ))
        })?;

        Ok(Self { channel })
    }

    /// Write a double-precision float to a VSS signal.
    ///
    /// Used for Vehicle.CurrentLocation.Latitude and .Longitude (09-REQ-1.1).
    pub async fn set_double(&self, path: &str, value: f64) -> Result<(), BrokerError> {
        use kuksav1::{datapoint, DataEntry, Datapoint, EntryUpdate, Field, SetRequest};
        use kuksav1::val_client::ValClient;

        let mut client = ValClient::new(self.channel.clone());
        let req = tonic::Request::new(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: path.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::Double(value)),
                    }),
                    actuator_target: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        });

        client
            .set(req)
            .await
            .map_err(|e| BrokerError(format!("Set RPC failed for '{}': {}", path, e)))?;

        Ok(())
    }

    /// Write a single-precision float to a VSS signal.
    ///
    /// Used for Vehicle.Speed (09-REQ-1.2).
    pub async fn set_float(&self, path: &str, value: f32) -> Result<(), BrokerError> {
        use kuksav1::{datapoint, DataEntry, Datapoint, EntryUpdate, Field, SetRequest};
        use kuksav1::val_client::ValClient;

        let mut client = ValClient::new(self.channel.clone());
        let req = tonic::Request::new(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: path.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::Float(value)),
                    }),
                    actuator_target: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        });

        client
            .set(req)
            .await
            .map_err(|e| BrokerError(format!("Set RPC failed for '{}': {}", path, e)))?;

        Ok(())
    }

    /// Write a boolean to a VSS signal.
    ///
    /// Used for Vehicle.Cabin.Door.Row1.DriverSide.IsOpen (09-REQ-1.3).
    pub async fn set_bool(&self, path: &str, value: bool) -> Result<(), BrokerError> {
        use kuksav1::{datapoint, DataEntry, Datapoint, EntryUpdate, Field, SetRequest};
        use kuksav1::val_client::ValClient;

        let mut client = ValClient::new(self.channel.clone());
        let req = tonic::Request::new(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: path.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::Bool(value)),
                    }),
                    actuator_target: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        });

        client
            .set(req)
            .await
            .map_err(|e| BrokerError(format!("Set RPC failed for '{}': {}", path, e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-09-21: Sensor Config Default
    // Requirement: 09-REQ-5.1
    #[test]
    fn test_config_default_broker_addr() {
        // Remove env var so we get the default.
        unsafe { std::env::remove_var("DATA_BROKER_ADDR") };
        let addr = get_broker_addr();
        assert_eq!(
            addr, "http://localhost:55556",
            "default DATA_BROKER_ADDR must be http://localhost:55556"
        );
    }

    // TS-09-21: Sensor Config Override
    // Requirement: 09-REQ-5.1
    #[test]
    fn test_config_env_overrides_default() {
        unsafe { std::env::set_var("DATA_BROKER_ADDR", "http://localhost:19999") };
        let addr = get_broker_addr();
        assert_eq!(addr, "http://localhost:19999");
        unsafe { std::env::remove_var("DATA_BROKER_ADDR") };
    }

    // TS-09-P1: Sensor Signal Type Correctness
    // Property 1 — signal paths must match VSS spec.
    #[test]
    fn test_location_sensor_signal_paths() {
        assert_eq!(SIGNAL_LATITUDE, "Vehicle.CurrentLocation.Latitude");
        assert_eq!(SIGNAL_LONGITUDE, "Vehicle.CurrentLocation.Longitude");
    }

    #[test]
    fn test_speed_sensor_signal_path() {
        assert_eq!(SIGNAL_SPEED, "Vehicle.Speed");
    }

    #[test]
    fn test_door_sensor_signal_path() {
        assert_eq!(SIGNAL_DOOR_OPEN, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen");
    }
}
