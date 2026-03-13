//! Speed sensor argument parsing.
//!
//! Provides helpers for parsing `--speed` CLI arguments
//! used by the `speed-sensor` binary.

/// VSS signal path for vehicle speed.
pub const SIGNAL: &str = crate::SPEED_SIGNAL;

/// Parsed speed sensor arguments.
#[derive(Debug, Clone)]
pub struct SpeedArgs {
    /// Speed value (f32/float).
    pub speed: f32,
    /// DATA_BROKER address.
    pub broker_addr: String,
}

/// Parse speed sensor CLI arguments.
///
/// Expects `--speed=<value>`. Optionally accepts `--broker-addr=<addr>`.
/// Returns `None` for `--help`/`-h`.
///
/// # Errors
///
/// Returns an error string if required arguments are missing or invalid.
pub fn parse_args(args: &[String]) -> Result<Option<SpeedArgs>, String> {
    let mut speed: Option<f32> = None;
    let mut broker_addr: Option<String> = None;

    for arg in args.iter().skip(1) {
        if arg == "--help" || arg == "-h" {
            return Ok(None);
        } else if let Some(val) = arg.strip_prefix("--speed=") {
            speed = Some(
                val.parse::<f32>()
                    .map_err(|e| format!("invalid --speed value: {}", e))?,
            );
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = Some(val.to_string());
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let speed = speed.ok_or_else(|| "required argument --speed is missing".to_string())?;

    let addr = broker_addr
        .or_else(|| std::env::var("DATA_BROKER_ADDR").ok())
        .unwrap_or_else(|| crate::DEFAULT_BROKER_ADDR.to_string());

    Ok(Some(SpeedArgs {
        speed,
        broker_addr: addr,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_speed() {
        let args = vec!["speed-sensor".to_string(), "--speed=60.5".to_string()];
        std::env::remove_var("DATA_BROKER_ADDR");
        let result = parse_args(&args).unwrap().unwrap();
        assert!((result.speed - 60.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_missing_speed() {
        let args = vec!["speed-sensor".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_help() {
        let args = vec!["speed-sensor".to_string(), "--help".to_string()];
        assert!(parse_args(&args).unwrap().is_none());
    }
}
