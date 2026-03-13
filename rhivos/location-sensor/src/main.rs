//! Mock location sensor CLI tool.
//!
//! Writes Vehicle.CurrentLocation.Latitude and Vehicle.CurrentLocation.Longitude
//! to DATA_BROKER via gRPC.

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
const LAT_SIGNAL: &str = "Vehicle.CurrentLocation.Latitude";
const LON_SIGNAL: &str = "Vehicle.CurrentLocation.Longitude";

const USAGE: &str = "Usage: location-sensor --lat=<LATITUDE> --lon=<LONGITUDE> [--broker-addr=<ADDR>]\n\nWrites Vehicle.CurrentLocation.Latitude and Vehicle.CurrentLocation.Longitude\nto DATA_BROKER via gRPC.\n\nFlags:\n  --lat=<value>          Latitude (required)\n  --lon=<value>          Longitude (required)\n  --broker-addr=<addr>   DATA_BROKER address (default: DATA_BROKER_ADDR env or http://localhost:55556)\n  --help, -h             Print this help message";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (lat, lon, broker_addr) = match parse_args(&args) {
        Ok(ParseResult::Args(lat, lon, addr)) => (lat, lon, addr),
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

    if let Err(e) = write_signals(&broker_addr, lat, lon).await {
        eprintln!("Error: failed to write to DATA_BROKER at {}: {}", broker_addr, e);
        process::exit(1);
    }

    println!("Set {}={} and {}={}", LAT_SIGNAL, lat, LON_SIGNAL, lon);
}

async fn write_signals(broker_addr: &str, lat: f64, lon: f64) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ValClient::connect(broker_addr.to_string()).await?;

    // Write latitude
    client.publish_value(PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(Signal::Path(LAT_SIGNAL.to_string())),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(Value {
                typed_value: Some(TypedValue::Double(lat)),
            }),
        }),
    }).await?;

    // Write longitude
    client.publish_value(PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(Signal::Path(LON_SIGNAL.to_string())),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(Value {
                typed_value: Some(TypedValue::Double(lon)),
            }),
        }),
    }).await?;

    Ok(())
}

/// Outcome of CLI argument parsing.
#[derive(Debug)]
enum ParseResult {
    /// Successfully parsed lat, lon, and broker address.
    Args(f64, f64, String),
    /// User requested --help.
    Help,
}

/// Parse CLI arguments.
/// Returns Err with a usage message if required arguments are missing.
fn parse_args(args: &[String]) -> Result<ParseResult, String> {
    let mut lat: Option<f64> = None;
    let mut lon: Option<f64> = None;
    let mut broker_addr: Option<String> = None;

    // Skip the program name (args[0])
    for arg in args.iter().skip(1) {
        if arg == "--help" || arg == "-h" {
            return Ok(ParseResult::Help);
        } else if let Some(val) = arg.strip_prefix("--lat=") {
            lat = Some(val.parse::<f64>().map_err(|e| format!("invalid --lat value: {}", e))?);
        } else if let Some(val) = arg.strip_prefix("--lon=") {
            lon = Some(val.parse::<f64>().map_err(|e| format!("invalid --lon value: {}", e))?);
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = Some(val.to_string());
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let lat = lat.ok_or_else(|| "required argument --lat is missing".to_string())?;
    let lon = lon.ok_or_else(|| "required argument --lon is missing".to_string())?;

    // Priority: --broker-addr flag > DATA_BROKER_ADDR env var > default
    let addr = broker_addr
        .or_else(|| std::env::var("DATA_BROKER_ADDR").ok())
        .unwrap_or_else(|| DEFAULT_BROKER_ADDR.to_string());

    Ok(ParseResult::Args(lat, lon, addr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that touch DATA_BROKER_ADDR env var.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper: unwrap ParseResult::Args or panic.
    fn unwrap_args(result: Result<ParseResult, String>) -> (f64, f64, String) {
        match result {
            Ok(ParseResult::Args(lat, lon, addr)) => (lat, lon, addr),
            Ok(ParseResult::Help) => panic!("Expected Args, got Help"),
            Err(e) => panic!("Expected Args, got Err: {}", e),
        }
    }

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
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATA_BROKER_ADDR");
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.1351".to_string(),
            "--lon=11.5820".to_string(),
        ];
        let (lat, lon, _addr) = unwrap_args(parse_args(&args));
        assert!((lat - 48.1351).abs() < f64::EPSILON, "Latitude mismatch");
        assert!((lon - 11.5820).abs() < f64::EPSILON, "Longitude mismatch");
    }

    /// TS-09-1: Custom broker address should be parsed (flag always wins).
    #[test]
    fn test_parses_custom_broker_addr() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
            "--broker-addr=http://192.168.1.10:55556".to_string(),
        ];
        let (_lat, _lon, addr) = unwrap_args(parse_args(&args));
        assert_eq!(addr, "http://192.168.1.10:55556");
    }

    /// TS-09-21: Default broker address should be http://localhost:55556.
    #[test]
    fn test_default_broker_addr() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATA_BROKER_ADDR");
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
        ];
        let (_lat, _lon, addr) = unwrap_args(parse_args(&args));
        assert_eq!(addr, "http://localhost:55556");
    }

    /// TS-09-21: DATA_BROKER_ADDR env var overrides default.
    #[test]
    fn test_env_var_overrides_default() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.1:55556");
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
        ];
        let (_lat, _lon, addr) = unwrap_args(parse_args(&args));
        assert_eq!(addr, "http://10.0.0.1:55556");
        std::env::remove_var("DATA_BROKER_ADDR");
    }

    /// TS-09-21: --broker-addr flag overrides DATA_BROKER_ADDR env var.
    #[test]
    fn test_flag_overrides_env_var() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.1:55556");
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
            "--broker-addr=http://custom:55556".to_string(),
        ];
        let (_lat, _lon, addr) = unwrap_args(parse_args(&args));
        assert_eq!(addr, "http://custom:55556");
        std::env::remove_var("DATA_BROKER_ADDR");
    }

    /// TS-09-6.1: --help should return Help variant.
    #[test]
    fn test_help_flag() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--help".to_string(),
        ];
        let result = parse_args(&args);
        assert!(matches!(result, Ok(ParseResult::Help)), "Expected Help for --help");
    }
}
