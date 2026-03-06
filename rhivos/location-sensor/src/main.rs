// Location sensor: writes GPS coordinates to DATA_BROKER via gRPC.
// Stub implementation -- to be completed in task group 2.

fn main() {
    eprintln!("location-sensor: not yet implemented");
    std::process::exit(1);
}

/// Parse CLI arguments and return (lat, lon, broker_addr).
/// Returns Err with a usage message if required args are missing.
pub fn parse_args(_args: &[String]) -> Result<(f64, f64, String), String> {
    // Stub: not yet implemented
    Err("not implemented".to_string())
}

/// Write latitude and longitude to DATA_BROKER via gRPC.
pub async fn write_location(
    _broker_addr: &str,
    _lat: f64,
    _lon: f64,
) -> Result<(), String> {
    // Stub: not yet implemented
    Err("not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-09-E1: Missing --lat and --lon should return an error.
    #[test]
    fn test_missing_lat_lon_exits_with_error() {
        let args: Vec<String> = vec!["location-sensor".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err(), "expected error when both --lat and --lon are missing");
    }

    /// TS-09-E1: Missing --lon should return an error.
    #[test]
    fn test_missing_lon_exits_with_error() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.1351".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_err(), "expected error when --lon is missing");
    }

    /// TS-09-1: Valid --lat and --lon should parse successfully and return correct values.
    #[test]
    fn test_parses_valid_lat_lon() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.1351".to_string(),
            "--lon=11.5820".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "expected successful parse with valid --lat and --lon");
        let (lat, lon, _addr) = result.unwrap();
        assert!((lat - 48.1351).abs() < 1e-6, "latitude mismatch");
        assert!((lon - 11.5820).abs() < 1e-6, "longitude mismatch");
    }

    /// TS-09-1: Verify the correct VSS signal paths are used.
    #[test]
    fn test_writes_correct_latitude_and_longitude() {
        // This test validates that write_location targets the correct VSS paths.
        // It will fail until the gRPC client is implemented.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(write_location("http://localhost:19999", 48.1351, 11.5820));
        // For now, we just verify the function exists and returns an error (stub).
        // Once implemented, this should succeed against a mock DATA_BROKER.
        assert!(result.is_err(), "stub should return error until implemented");
    }
}
