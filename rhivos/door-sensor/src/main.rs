//! Mock door sensor CLI tool.
//!
//! Writes Vehicle.Cabin.Door.Row1.DriverSide.IsOpen to DATA_BROKER via gRPC.

use std::process;

fn main() {
    // TODO: implement CLI parsing and DATA_BROKER write
    eprintln!("door-sensor: not yet implemented");
    process::exit(1);
}

/// Parse CLI arguments and return (is_open, broker_addr).
/// Returns Err with a usage message if neither --open nor --closed is provided.
#[allow(dead_code)] // Stub: used by tests, will be called from main() when implemented
fn parse_args(_args: &[String]) -> Result<(bool, String), String> {
    // Stub: always fails
    Err("not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-09-E1: Missing --open or --closed should produce an error.
    #[test]
    fn test_missing_open_or_closed_exits_with_error() {
        let args: Vec<String> = vec!["door-sensor".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err(), "Expected error when --open or --closed is missing");
        let err = result.unwrap_err();
        assert!(
            err.to_lowercase().contains("open") || err.to_lowercase().contains("closed") || err.to_lowercase().contains("required"),
            "Error message should mention missing argument: {err}"
        );
    }

    /// TS-09-3: --open should parse as true.
    #[test]
    fn test_writes_open_true() {
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--open".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse with --open");
        let (is_open, _addr) = result.unwrap();
        assert!(is_open, "Expected is_open=true for --open");
    }

    /// TS-09-4: --closed should parse as false.
    #[test]
    fn test_writes_closed_false() {
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--closed".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse with --closed");
        let (is_open, _addr) = result.unwrap();
        assert!(!is_open, "Expected is_open=false for --closed");
    }

    /// TS-09-3/4: Default broker address should be http://localhost:55556.
    #[test]
    fn test_default_broker_addr() {
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--open".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse");
        let (_is_open, addr) = result.unwrap();
        assert_eq!(addr, "http://localhost:55556");
    }
}
