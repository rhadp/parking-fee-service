//! Shared code for mock sensor binaries.
//!
//! This module provides:
//! - Configuration helpers (DATA_BROKER_ADDR resolution)
//! - VSS signal path constants
//! - Print-usage helper

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

/// Print usage information for a mock sensor binary.
pub fn print_usage(sensor_name: &str) {
    println!("{sensor_name} v0.1.0 - Mock sensor for vehicle signals");
    println!();
    println!("Usage: {sensor_name} [options]");
    println!();
    println!("This is a skeleton implementation. See spec 09 for full functionality.");
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
