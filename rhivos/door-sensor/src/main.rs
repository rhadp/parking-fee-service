//! Mock door sensor CLI tool.
//!
//! Writes Vehicle.Cabin.Door.Row1.DriverSide.IsOpen to DATA_BROKER via gRPC.

use std::process;

mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

use kuksa::val::v2::{
    val_client::ValClient, PublishValueRequest, SignalId,
    signal_id::Signal, Datapoint, Value, value::TypedValue,
};

const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";
const DOOR_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

const USAGE: &str = "Usage: door-sensor <--open|--closed> [--broker-addr=<ADDR>]\n\nWrites Vehicle.Cabin.Door.Row1.DriverSide.IsOpen to DATA_BROKER via gRPC.\n\nFlags:\n  --open                 Set door state to open (true)\n  --closed               Set door state to closed (false)\n  --broker-addr=<addr>   DATA_BROKER address (default: DATA_BROKER_ADDR env or http://localhost:55556)\n  --help, -h             Print this help message";

/// Outcome of CLI argument parsing.
#[derive(Debug)]
enum ParseResult {
    /// Successfully parsed door state and broker address.
    Args(bool, String),
    /// User requested --help.
    Help,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (is_open, broker_addr) = match parse_args(&args) {
        Ok(ParseResult::Args(is_open, addr)) => (is_open, addr),
        Ok(ParseResult::Help) => {
            println!("{}", USAGE);
            process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("{}", USAGE);
            process::exit(1);
        }
    };

    if let Err(e) = write_signal(&broker_addr, is_open).await {
        eprintln!("Error: failed to write to DATA_BROKER at {}: {}", broker_addr, e);
        process::exit(1);
    }

    let state = if is_open { "open (true)" } else { "closed (false)" };
    println!("Set {}={}", DOOR_SIGNAL, state);
}

async fn write_signal(broker_addr: &str, is_open: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ValClient::connect(broker_addr.to_string()).await?;

    client.publish_value(PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(Signal::Path(DOOR_SIGNAL.to_string())),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(Value {
                typed_value: Some(TypedValue::Bool(is_open)),
            }),
        }),
    }).await?;

    Ok(())
}

/// Parse CLI arguments.
/// Returns Err with a usage message if neither --open nor --closed is provided.
fn parse_args(args: &[String]) -> Result<ParseResult, String> {
    let mut is_open: Option<bool> = None;
    let mut broker_addr: Option<String> = None;

    // Skip the program name (args[0])
    for arg in args.iter().skip(1) {
        if arg == "--help" || arg == "-h" {
            return Ok(ParseResult::Help);
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

    let is_open = is_open.ok_or_else(|| "required argument --open or --closed is missing".to_string())?;

    // Priority: --broker-addr flag > DATA_BROKER_ADDR env var > default
    let addr = broker_addr
        .or_else(|| std::env::var("DATA_BROKER_ADDR").ok())
        .unwrap_or_else(|| DEFAULT_BROKER_ADDR.to_string());

    Ok(ParseResult::Args(is_open, addr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that touch DATA_BROKER_ADDR env var.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper: unwrap ParseResult::Args or panic.
    fn unwrap_args(result: Result<ParseResult, String>) -> (bool, String) {
        match result {
            Ok(ParseResult::Args(is_open, addr)) => (is_open, addr),
            Ok(ParseResult::Help) => panic!("Expected Args, got Help"),
            Err(e) => panic!("Expected Args, got Err: {}", e),
        }
    }

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
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATA_BROKER_ADDR");
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--open".to_string(),
        ];
        let (is_open, _addr) = unwrap_args(parse_args(&args));
        assert!(is_open, "Expected is_open=true for --open");
    }

    /// TS-09-4: --closed should parse as false.
    #[test]
    fn test_writes_closed_false() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATA_BROKER_ADDR");
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--closed".to_string(),
        ];
        let (is_open, _addr) = unwrap_args(parse_args(&args));
        assert!(!is_open, "Expected is_open=false for --closed");
    }

    /// TS-09-21: Default broker address should be http://localhost:55556.
    #[test]
    fn test_default_broker_addr() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATA_BROKER_ADDR");
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--open".to_string(),
        ];
        let (_is_open, addr) = unwrap_args(parse_args(&args));
        assert_eq!(addr, "http://localhost:55556");
    }

    /// TS-09-21: DATA_BROKER_ADDR env var overrides default.
    #[test]
    fn test_env_var_overrides_default() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.1:55556");
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--open".to_string(),
        ];
        let (_is_open, addr) = unwrap_args(parse_args(&args));
        assert_eq!(addr, "http://10.0.0.1:55556");
        std::env::remove_var("DATA_BROKER_ADDR");
    }

    /// TS-09-6.1: --help should return Help variant.
    #[test]
    fn test_help_flag() {
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--help".to_string(),
        ];
        let result = parse_args(&args);
        assert!(matches!(result, Ok(ParseResult::Help)), "Expected Help for --help");
    }
}
