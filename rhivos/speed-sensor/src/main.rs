//! Mock speed sensor CLI tool.
//!
//! Writes Vehicle.Speed to DATA_BROKER via gRPC.

use std::process;

fn main() {
    // TODO: implement CLI parsing and DATA_BROKER write
    eprintln!("speed-sensor: not yet implemented");
    process::exit(1);
}

/// Parse CLI arguments and return (speed, broker_addr).
/// Returns Err with a usage message if required arguments are missing.
#[allow(dead_code)] // Stub: used by tests, will be called from main() when implemented
fn parse_args(_args: &[String]) -> Result<(f32, String), String> {
    // Stub: always fails
    Err("not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-09-E1: Missing --speed should produce an error.
    #[test]
    fn test_missing_speed_exits_with_error() {
        let args: Vec<String> = vec!["speed-sensor".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err(), "Expected error when --speed is missing");
        let err = result.unwrap_err();
        assert!(
            err.to_lowercase().contains("speed") || err.to_lowercase().contains("required"),
            "Error message should mention missing argument: {err}"
        );
    }

    /// TS-09-2: Valid --speed should parse correctly.
    #[test]
    fn test_parses_valid_speed() {
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=50.5".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse with valid args");
        let (speed, _addr) = result.unwrap();
        assert!((speed - 50.5).abs() < f32::EPSILON, "Speed mismatch");
    }

    /// TS-09-2: Default broker address should be http://localhost:55556.
    #[test]
    fn test_default_broker_addr() {
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=0.0".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse");
        let (_speed, addr) = result.unwrap();
        assert_eq!(addr, "http://localhost:55556");
    }
}
