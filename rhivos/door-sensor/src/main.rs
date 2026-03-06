// Door sensor: writes door open/closed state to DATA_BROKER via gRPC.
// Stub implementation -- to be completed in task group 2.

fn main() {
    eprintln!("door-sensor: not yet implemented");
    std::process::exit(1);
}

/// Parse CLI arguments and return (is_open, broker_addr).
/// Returns Err with a usage message if neither --open nor --closed is provided.
pub fn parse_args(_args: &[String]) -> Result<(bool, String), String> {
    // Stub: not yet implemented
    Err("not implemented".to_string())
}

/// Write door state to DATA_BROKER via gRPC.
pub async fn write_door_state(
    _broker_addr: &str,
    _is_open: bool,
) -> Result<(), String> {
    // Stub: not yet implemented
    Err("not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-09-E1: Missing --open or --closed should return an error.
    #[test]
    fn test_missing_open_or_closed_exits_with_error() {
        let args: Vec<String> = vec!["door-sensor".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err(), "expected error when neither --open nor --closed is provided");
    }

    /// TS-09-3: --open should parse as true.
    #[test]
    fn test_parses_open_as_true() {
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--open".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "expected successful parse with --open");
        let (is_open, _addr) = result.unwrap();
        assert!(is_open, "expected is_open to be true for --open");
    }

    /// TS-09-4: --closed should parse as false.
    #[test]
    fn test_parses_closed_as_false() {
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--closed".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "expected successful parse with --closed");
        let (is_open, _addr) = result.unwrap();
        assert!(!is_open, "expected is_open to be false for --closed");
    }

    /// TS-09-3: Verify the correct VSS signal path is used (open).
    #[test]
    fn test_writes_open_true() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(write_door_state("http://localhost:19999", true));
        assert!(result.is_err(), "stub should return error until implemented");
    }

    /// TS-09-4: Verify the correct VSS signal path is used (closed).
    #[test]
    fn test_writes_closed_false() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(write_door_state("http://localhost:19999", false));
        assert!(result.is_err(), "stub should return error until implemented");
    }
}
