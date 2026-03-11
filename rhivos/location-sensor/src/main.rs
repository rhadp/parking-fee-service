//! Mock location sensor CLI tool.
//!
//! Writes Vehicle.CurrentLocation.Latitude and Vehicle.CurrentLocation.Longitude
//! to DATA_BROKER via gRPC.

use std::process;

fn main() {
    // TODO: implement CLI parsing and DATA_BROKER write
    eprintln!("location-sensor: not yet implemented");
    process::exit(1);
}

/// Parse CLI arguments and return (lat, lon, broker_addr).
/// Returns Err with a usage message if required arguments are missing.
#[allow(dead_code)] // Stub: used by tests, will be called from main() when implemented
fn parse_args(_args: &[String]) -> Result<(f64, f64, String), String> {
    // Stub: always fails
    Err("not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-09-E1: Missing --lat and --lon should produce an error.
    #[test]
    fn test_missing_lat_lon_exits_with_error() {
        let args: Vec<String> = vec!["location-sensor".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err(), "Expected error when --lat and --lon are missing");
        let err = result.unwrap_err();
        assert!(
            err.to_lowercase().contains("lat") || err.to_lowercase().contains("required"),
            "Error message should mention missing argument: {err}"
        );
    }

    /// TS-09-E1: Missing --lon (only --lat provided) should produce an error.
    #[test]
    fn test_missing_lon_exits_with_error() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.1351".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_err(), "Expected error when --lon is missing");
    }

    /// TS-09-1: Valid --lat and --lon should parse correctly.
    #[test]
    fn test_parses_valid_lat_lon() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.1351".to_string(),
            "--lon=11.5820".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse with valid args");
        let (lat, lon, _addr) = result.unwrap();
        assert!((lat - 48.1351).abs() < f64::EPSILON, "Latitude mismatch");
        assert!((lon - 11.5820).abs() < f64::EPSILON, "Longitude mismatch");
    }

    /// TS-09-1: Custom broker address should be parsed.
    #[test]
    fn test_parses_custom_broker_addr() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
            "--broker-addr=http://192.168.1.10:55556".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse");
        let (_lat, _lon, addr) = result.unwrap();
        assert_eq!(addr, "http://192.168.1.10:55556");
    }

    /// TS-09-1: Default broker address should be http://localhost:55556.
    #[test]
    fn test_default_broker_addr() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse");
        let (_lat, _lon, addr) = result.unwrap();
        assert_eq!(addr, "http://localhost:55556");
    }
}
