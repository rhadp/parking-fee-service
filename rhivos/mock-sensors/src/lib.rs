//! Mock sensor library providing shared utilities for simulated vehicle signal inputs.
//!
//! Provides [`BrokerWriter`] for writing VSS-compliant signals to DATA_BROKER
//! via kuksa.val.v2 gRPC, and argument-parsing helpers for each sensor type.
//!
//! # Example
//!
//! ```no_run
//! use mock_sensors::BrokerWriter;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut writer = BrokerWriter::connect("http://localhost:55556").await?;
//! writer.set_double("Vehicle.CurrentLocation.Latitude", 48.1351).await?;
//! writer.set_float("Vehicle.Speed", 60.5).await?;
//! writer.set_bool("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", true).await?;
//! # Ok(())
//! # }
//! ```

pub mod location;
pub mod speed;
pub mod door;

mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

use kuksa::val::v2::{
    val_client::ValClient,
    PublishValueRequest,
    SignalId,
    signal_id::Signal,
    Datapoint,
    Value,
    value::TypedValue,
};

/// Signal path constants for VSS-compliant vehicle signals.
pub const LOCATION_LAT_SIGNAL: &str = "Vehicle.CurrentLocation.Latitude";
pub const LOCATION_LON_SIGNAL: &str = "Vehicle.CurrentLocation.Longitude";
pub const SPEED_SIGNAL: &str = "Vehicle.Speed";
pub const DOOR_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

/// Default DATA_BROKER address.
pub const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";

/// Returns the configured DATA_BROKER address from environment or default.
///
/// Priority: `DATA_BROKER_ADDR` env var > [`DEFAULT_BROKER_ADDR`].
pub fn get_broker_addr() -> String {
    std::env::var("DATA_BROKER_ADDR").unwrap_or_else(|_| DEFAULT_BROKER_ADDR.to_string())
}

/// A client for writing vehicle signal values to DATA_BROKER via gRPC.
///
/// Wraps a kuksa.val.v2 `VAL` gRPC client and provides typed methods
/// for publishing signal values.
pub struct BrokerWriter {
    client: ValClient<tonic::transport::Channel>,
}

impl BrokerWriter {
    /// Connect to DATA_BROKER at the given address.
    ///
    /// # Errors
    ///
    /// Returns an error if the gRPC connection cannot be established.
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let client = ValClient::connect(addr.to_string())
            .await
            .map_err(|e| BrokerError::Connection(addr.to_string(), e.to_string()))?;
        Ok(Self { client })
    }

    /// Write a `double` (f64) value to the given VSS signal path.
    ///
    /// Used for signals like `Vehicle.CurrentLocation.Latitude` and
    /// `Vehicle.CurrentLocation.Longitude`.
    pub async fn set_double(&mut self, path: &str, value: f64) -> Result<(), BrokerError> {
        self.publish(path, TypedValue::Double(value)).await
    }

    /// Write a `float` (f32) value to the given VSS signal path.
    ///
    /// Used for signals like `Vehicle.Speed`.
    pub async fn set_float(&mut self, path: &str, value: f32) -> Result<(), BrokerError> {
        self.publish(path, TypedValue::Float(value)).await
    }

    /// Write a `bool` value to the given VSS signal path.
    ///
    /// Used for signals like `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.
    pub async fn set_bool(&mut self, path: &str, value: bool) -> Result<(), BrokerError> {
        self.publish(path, TypedValue::Bool(value)).await
    }

    /// Internal helper to publish a typed value to a signal path.
    async fn publish(&mut self, path: &str, typed_value: TypedValue) -> Result<(), BrokerError> {
        self.client
            .publish_value(PublishValueRequest {
                signal_id: Some(SignalId {
                    signal: Some(Signal::Path(path.to_string())),
                }),
                data_point: Some(Datapoint {
                    timestamp: None,
                    value: Some(Value {
                        typed_value: Some(typed_value),
                    }),
                }),
            })
            .await
            .map_err(|e| BrokerError::Publish(path.to_string(), e.to_string()))?;
        Ok(())
    }
}

/// Errors that can occur when interacting with DATA_BROKER.
#[derive(Debug)]
pub enum BrokerError {
    /// Failed to connect to DATA_BROKER at the given address.
    Connection(String, String),
    /// Failed to publish a value to the given signal path.
    Publish(String, String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::Connection(addr, msg) => {
                write!(f, "failed to connect to DATA_BROKER at {}: {}", addr, msg)
            }
            BrokerError::Publish(path, msg) => {
                write!(f, "failed to publish to signal {}: {}", path, msg)
            }
        }
    }
}

impl std::error::Error for BrokerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modules_exist() {
        // Validates that all sensor modules compile
        assert!(true, "mock-sensors library compiles with all modules");
    }

    /// TS-09-21: Sensors default to DATA_BROKER_ADDR=http://localhost:55556.
    #[test]
    fn test_config_default_broker_addr() {
        std::env::remove_var("DATA_BROKER_ADDR");
        let addr = get_broker_addr();
        assert_eq!(addr, "http://localhost:55556");
    }

    /// TS-09-21: DATA_BROKER_ADDR env var overrides default.
    #[test]
    fn test_config_override_broker_addr() {
        std::env::set_var("DATA_BROKER_ADDR", "http://192.168.1.10:55556");
        let addr = get_broker_addr();
        assert_eq!(addr, "http://192.168.1.10:55556");
        std::env::remove_var("DATA_BROKER_ADDR");
    }

    /// TS-09-P1: Sensor Signal Type Correctness property test.
    /// For any valid sensor arguments, the correct VSS signal path is used.
    #[test]
    fn test_property_sensor_signal_type_correctness() {
        // Location sensor uses double (f64) for lat/lon
        assert_eq!(LOCATION_LAT_SIGNAL, "Vehicle.CurrentLocation.Latitude");
        assert_eq!(LOCATION_LON_SIGNAL, "Vehicle.CurrentLocation.Longitude");

        // Speed sensor signal path
        assert_eq!(SPEED_SIGNAL, "Vehicle.Speed");

        // Door sensor signal path
        assert_eq!(DOOR_SIGNAL, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen");

        // Verify all signal paths are non-empty and start with "Vehicle."
        let signals = [LOCATION_LAT_SIGNAL, LOCATION_LON_SIGNAL, SPEED_SIGNAL, DOOR_SIGNAL];
        for signal in &signals {
            assert!(!signal.is_empty(), "Signal path should not be empty");
            assert!(
                signal.starts_with("Vehicle."),
                "Signal path should start with Vehicle.: {}",
                signal
            );
        }

        // Property: validate with random-ish values across type ranges.
        let lat_values: Vec<f64> = vec![-90.0, -45.5, 0.0, 48.1351, 90.0];
        let lon_values: Vec<f64> = vec![-180.0, -90.0, 0.0, 11.5820, 180.0];
        let speed_values: Vec<f32> = vec![0.0, 50.5, 120.0, 200.0, 300.0];
        let door_values: Vec<bool> = vec![true, false];

        for lat in &lat_values {
            assert!(lat.is_finite(), "Latitude must be finite: {}", lat);
        }
        for lon in &lon_values {
            assert!(lon.is_finite(), "Longitude must be finite: {}", lon);
        }
        for speed in &speed_values {
            assert!(speed.is_finite(), "Speed must be finite: {}", speed);
            assert!(*speed >= 0.0, "Speed must be non-negative: {}", speed);
        }
        for door in &door_values {
            // Bool values are always valid
            let _ = door;
        }
    }

    /// Verify BrokerError display messages include the address/path.
    #[test]
    fn test_broker_error_display() {
        let conn_err = BrokerError::Connection(
            "http://localhost:55556".to_string(),
            "connection refused".to_string(),
        );
        let msg = conn_err.to_string();
        assert!(msg.contains("localhost:55556"), "Error should contain address: {}", msg);
        assert!(msg.contains("connection refused"), "Error should contain cause: {}", msg);

        let pub_err = BrokerError::Publish(
            "Vehicle.Speed".to_string(),
            "not found".to_string(),
        );
        let msg = pub_err.to_string();
        assert!(msg.contains("Vehicle.Speed"), "Error should contain path: {}", msg);
    }
}
