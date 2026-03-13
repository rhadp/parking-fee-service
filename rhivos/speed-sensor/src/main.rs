//! Mock speed sensor CLI tool.
//!
//! Writes Vehicle.Speed to DATA_BROKER via gRPC.

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
const SPEED_SIGNAL: &str = "Vehicle.Speed";

const USAGE: &str = "Usage: speed-sensor --speed=<SPEED> [--broker-addr=<ADDR>]\n\nWrites Vehicle.Speed to DATA_BROKER via gRPC.\n\nFlags:\n  --speed=<value>        Speed value (required)\n  --broker-addr=<addr>   DATA_BROKER address (default: DATA_BROKER_ADDR env or http://localhost:55556)\n  --help, -h             Print this help message";

/// Outcome of CLI argument parsing.
#[derive(Debug)]
enum ParseResult {
    /// Successfully parsed speed and broker address.
    Args(f32, String),
    /// User requested --help.
    Help,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (speed, broker_addr) = match parse_args(&args) {
        Ok(ParseResult::Args(speed, addr)) => (speed, addr),
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

    if let Err(e) = write_signal(&broker_addr, speed).await {
        eprintln!("Error: failed to write to DATA_BROKER at {}: {}", broker_addr, e);
        process::exit(1);
    }

    println!("Set {}={}", SPEED_SIGNAL, speed);
}

async fn write_signal(broker_addr: &str, speed: f32) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ValClient::connect(broker_addr.to_string()).await?;

    client.publish_value(PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(Signal::Path(SPEED_SIGNAL.to_string())),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(Value {
                typed_value: Some(TypedValue::Float(speed)),
            }),
        }),
    }).await?;

    Ok(())
}

/// Parse CLI arguments.
/// Returns Err with a usage message if required arguments are missing.
fn parse_args(args: &[String]) -> Result<ParseResult, String> {
    let mut speed: Option<f32> = None;
    let mut broker_addr: Option<String> = None;

    // Skip the program name (args[0])
    for arg in args.iter().skip(1) {
        if arg == "--help" || arg == "-h" {
            return Ok(ParseResult::Help);
        } else if let Some(val) = arg.strip_prefix("--speed=") {
            speed = Some(val.parse::<f32>().map_err(|e| format!("invalid --speed value: {}", e))?);
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = Some(val.to_string());
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let speed = speed.ok_or_else(|| "required argument --speed is missing".to_string())?;

    // Priority: --broker-addr flag > DATA_BROKER_ADDR env var > default
    let addr = broker_addr
        .or_else(|| std::env::var("DATA_BROKER_ADDR").ok())
        .unwrap_or_else(|| DEFAULT_BROKER_ADDR.to_string());

    Ok(ParseResult::Args(speed, addr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that touch DATA_BROKER_ADDR env var.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper: unwrap ParseResult::Args or panic.
    fn unwrap_args(result: Result<ParseResult, String>) -> (f32, String) {
        match result {
            Ok(ParseResult::Args(speed, addr)) => (speed, addr),
            Ok(ParseResult::Help) => panic!("Expected Args, got Help"),
            Err(e) => panic!("Expected Args, got Err: {}", e),
        }
    }

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
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATA_BROKER_ADDR");
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=50.5".to_string(),
        ];
        let (speed, _addr) = unwrap_args(parse_args(&args));
        assert!((speed - 50.5).abs() < f32::EPSILON, "Speed mismatch");
    }

    /// TS-09-21: Default broker address should be http://localhost:55556.
    #[test]
    fn test_default_broker_addr() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATA_BROKER_ADDR");
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=0.0".to_string(),
        ];
        let (_speed, addr) = unwrap_args(parse_args(&args));
        assert_eq!(addr, "http://localhost:55556");
    }

    /// TS-09-21: DATA_BROKER_ADDR env var overrides default.
    #[test]
    fn test_env_var_overrides_default() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.1:55556");
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=0.0".to_string(),
        ];
        let (_speed, addr) = unwrap_args(parse_args(&args));
        assert_eq!(addr, "http://10.0.0.1:55556");
        std::env::remove_var("DATA_BROKER_ADDR");
    }

    /// TS-09-6.1: --help should return Help variant.
    #[test]
    fn test_help_flag() {
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--help".to_string(),
        ];
        let result = parse_args(&args);
        assert!(matches!(result, Ok(ParseResult::Help)), "Expected Help for --help");
    }
}
