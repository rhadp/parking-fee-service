//! Door sensor argument parsing.
//!
//! Provides helpers for parsing `--open`/`--closed` CLI arguments
//! used by the `door-sensor` binary.

/// VSS signal path for driver-side door open state.
pub const SIGNAL: &str = crate::DOOR_SIGNAL;

/// Parsed door sensor arguments.
#[derive(Debug, Clone)]
pub struct DoorArgs {
    /// Whether the door is open (`true`) or closed (`false`).
    pub is_open: bool,
    /// DATA_BROKER address.
    pub broker_addr: String,
}

/// Parse door sensor CLI arguments.
///
/// Expects `--open` or `--closed`. Optionally accepts `--broker-addr=<addr>`.
/// Returns `None` for `--help`/`-h`.
///
/// # Errors
///
/// Returns an error string if neither `--open` nor `--closed` is provided.
pub fn parse_args(args: &[String]) -> Result<Option<DoorArgs>, String> {
    let mut is_open: Option<bool> = None;
    let mut broker_addr: Option<String> = None;

    for arg in args.iter().skip(1) {
        if arg == "--help" || arg == "-h" {
            return Ok(None);
        } else if arg == "--open" {
            is_open = Some(true);
        } else if arg == "--closed" {
            is_open = Some(false);
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = Some(val.to_string());
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let is_open =
        is_open.ok_or_else(|| "required argument --open or --closed is missing".to_string())?;

    let addr = broker_addr
        .or_else(|| std::env::var("DATA_BROKER_ADDR").ok())
        .unwrap_or_else(|| crate::DEFAULT_BROKER_ADDR.to_string());

    Ok(Some(DoorArgs {
        is_open,
        broker_addr: addr,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_open() {
        let args = vec!["door-sensor".to_string(), "--open".to_string()];
        std::env::remove_var("DATA_BROKER_ADDR");
        let result = parse_args(&args).unwrap().unwrap();
        assert!(result.is_open);
    }

    #[test]
    fn test_parse_closed() {
        let args = vec!["door-sensor".to_string(), "--closed".to_string()];
        std::env::remove_var("DATA_BROKER_ADDR");
        let result = parse_args(&args).unwrap().unwrap();
        assert!(!result.is_open);
    }

    #[test]
    fn test_parse_missing_state() {
        let args = vec!["door-sensor".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_help() {
        let args = vec!["door-sensor".to_string(), "--help".to_string()];
        assert!(parse_args(&args).unwrap().is_none());
    }
}
