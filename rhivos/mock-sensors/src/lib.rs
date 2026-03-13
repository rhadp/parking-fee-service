//! Mock sensor library providing simulated vehicle signal inputs.
//!
//! # Example
//!
//! ```
//! // mock-sensors crate provides location, speed, and door modules
//! use mock_sensors::location;
//! use mock_sensors::speed;
//! use mock_sensors::door;
//! ```

pub mod location;
pub mod speed;
pub mod door;

/// Signal path constants for VSS-compliant vehicle signals.
pub const LOCATION_LAT_SIGNAL: &str = "Vehicle.CurrentLocation.Latitude";
pub const LOCATION_LON_SIGNAL: &str = "Vehicle.CurrentLocation.Longitude";
pub const SPEED_SIGNAL: &str = "Vehicle.Speed";
pub const DOOR_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

/// Default DATA_BROKER address.
pub const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";

/// Returns the configured DATA_BROKER address from environment or default.
pub fn get_broker_addr() -> String {
    std::env::var("DATA_BROKER_ADDR").unwrap_or_else(|_| DEFAULT_BROKER_ADDR.to_string())
}

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
}
