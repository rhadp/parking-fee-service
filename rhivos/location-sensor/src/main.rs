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

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (lat, lon, broker_addr) = match parse_args(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Usage: location-sensor --lat=<LATITUDE> --lon=<LONGITUDE> [--broker-addr=<ADDR>]");
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

/// Parse CLI arguments and return (lat, lon, broker_addr).
/// Returns Err with a usage message if required arguments are missing.
fn parse_args(args: &[String]) -> Result<(f64, f64, String), String> {
    let mut lat: Option<f64> = None;
    let mut lon: Option<f64> = None;
    let mut broker_addr = DEFAULT_BROKER_ADDR.to_string();

    // Skip the program name (args[0])
    for arg in args.iter().skip(1) {
        if let Some(val) = arg.strip_prefix("--lat=") {
            lat = Some(val.parse::<f64>().map_err(|e| format!("invalid --lat value: {}", e))?);
        } else if let Some(val) = arg.strip_prefix("--lon=") {
            lon = Some(val.parse::<f64>().map_err(|e| format!("invalid --lon value: {}", e))?);
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = val.to_string();
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let lat = lat.ok_or_else(|| "required argument --lat is missing".to_string())?;
    let lon = lon.ok_or_else(|| "required argument --lon is missing".to_string())?;

    Ok((lat, lon, broker_addr))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.1351".to_string(),
            "--lon=11.5820".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse with valid args");
        let (lat, lon, _addr) = result.unwrap();
        assert!((lat - 48.1351).abs() < f64::EPSILON, "Latitude mismatch");
        assert!((lon - 11.5820).abs() < f64::EPSILON, "Longitude mismatch");
    }

    /// TS-09-1: Custom broker address should be parsed.
    #[test]
    fn test_parses_custom_broker_addr() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
            "--broker-addr=http://192.168.1.10:55556".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse");
        let (_lat, _lon, addr) = result.unwrap();
        assert_eq!(addr, "http://192.168.1.10:55556");
    }

    /// TS-09-1: Default broker address should be http://localhost:55556.
    #[test]
    fn test_default_broker_addr() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse");
        let (_lat, _lon, addr) = result.unwrap();
        assert_eq!(addr, "http://localhost:55556");
    }
}
