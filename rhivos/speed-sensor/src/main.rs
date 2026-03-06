// Speed sensor: writes vehicle speed to DATA_BROKER via gRPC.
// Stub implementation -- to be completed in task group 2.

fn main() {
    eprintln!("speed-sensor: not yet implemented");
    std::process::exit(1);
}

/// Parse CLI arguments and return (speed, broker_addr).
/// Returns Err with a usage message if required args are missing.
pub fn parse_args(_args: &[String]) -> Result<(f32, String), String> {
    // Stub: not yet implemented
    Err("not implemented".to_string())
}

/// Write speed to DATA_BROKER via gRPC.
pub async fn write_speed(
    _broker_addr: &str,
    _speed: f32,
) -> Result<(), String> {
    // Stub: not yet implemented
    Err("not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-09-E1: Missing --speed should return an error.
    #[test]
    fn test_missing_speed_exits_with_error() {
        let args: Vec<String> = vec!["speed-sensor".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err(), "expected error when --speed is missing");
    }

    /// TS-09-2: Valid --speed should parse successfully and return correct value.
    #[test]
    fn test_parses_valid_speed() {
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=50.5".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "expected successful parse with valid --speed");
        let (speed, _addr) = result.unwrap();
        assert!((speed - 50.5).abs() < 1e-3, "speed mismatch");
    }

    /// TS-09-2: Verify the correct VSS signal path is used.
    #[test]
    fn test_writes_correct_speed() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(write_speed("http://localhost:19999", 50.5));
        assert!(result.is_err(), "stub should return error until implemented");
    }
}
