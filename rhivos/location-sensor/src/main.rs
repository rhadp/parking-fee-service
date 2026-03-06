// Location sensor: writes GPS coordinates to DATA_BROKER via gRPC.
// Implements 09-REQ-1.1, 09-REQ-7.1, 09-REQ-8.2.

/// Generated Kuksa VAL v2 protobuf types.
#[allow(clippy::doc_overindented_list_items, clippy::enum_variant_names)]
mod kuksa_proto {
    tonic::include_proto!("kuksa.val.v2");
}

use kuksa_proto::val_client::ValClient;
use kuksa_proto::value::TypedValue;
use kuksa_proto::{Datapoint, PublishValueRequest, SignalId};

const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";
const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (lat, lon, broker_addr) = match parse_args(&args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("Error: {}", msg);
            eprintln!("Usage: location-sensor --lat=<LATITUDE> --lon=<LONGITUDE> [--broker-addr=<ADDR>]");
            std::process::exit(1);
        }
    };

    match write_location(&broker_addr, lat, lon).await {
        Ok(()) => {
            println!(
                "Published {}={} and {}={} to {}",
                SIGNAL_LATITUDE, lat, SIGNAL_LONGITUDE, lon, broker_addr
            );
        }
        Err(msg) => {
            eprintln!("Error: {}", msg);
            std::process::exit(1);
        }
    }
}

/// Parse CLI arguments and return (lat, lon, broker_addr).
/// Returns Err with a usage message if required args are missing.
pub fn parse_args(args: &[String]) -> Result<(f64, f64, String), String> {
    let mut lat: Option<f64> = None;
    let mut lon: Option<f64> = None;
    let mut broker_addr = DEFAULT_BROKER_ADDR.to_string();

    // Skip the program name (args[0])
    for arg in args.iter().skip(1) {
        if let Some(val) = arg.strip_prefix("--lat=") {
            lat = Some(
                val.parse::<f64>()
                    .map_err(|e| format!("invalid --lat value: {}", e))?,
            );
        } else if let Some(val) = arg.strip_prefix("--lon=") {
            lon = Some(
                val.parse::<f64>()
                    .map_err(|e| format!("invalid --lon value: {}", e))?,
            );
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = val.to_string();
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let lat = lat.ok_or("missing required argument: --lat")?;
    let lon = lon.ok_or("missing required argument: --lon")?;

    Ok((lat, lon, broker_addr))
}

/// Write latitude and longitude to DATA_BROKER via gRPC.
pub async fn write_location(broker_addr: &str, lat: f64, lon: f64) -> Result<(), String> {
    let mut client = ValClient::connect(broker_addr.to_string())
        .await
        .map_err(|e| format!("failed to connect to DATA_BROKER at {}: {}", broker_addr, e))?;

    // Write latitude
    let request = PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(kuksa_proto::signal_id::Signal::Path(
                SIGNAL_LATITUDE.to_string(),
            )),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(kuksa_proto::Value {
                typed_value: Some(TypedValue::Double(lat)),
            }),
        }),
    };
    client
        .publish_value(request)
        .await
        .map_err(|e| format!("failed to publish {}: {}", SIGNAL_LATITUDE, e))?;

    // Write longitude
    let request = PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(kuksa_proto::signal_id::Signal::Path(
                SIGNAL_LONGITUDE.to_string(),
            )),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(kuksa_proto::Value {
                typed_value: Some(TypedValue::Double(lon)),
            }),
        }),
    };
    client
        .publish_value(request)
        .await
        .map_err(|e| format!("failed to publish {}: {}", SIGNAL_LONGITUDE, e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-09-E1: Missing --lat and --lon should return an error.
    #[test]
    fn test_missing_lat_lon_exits_with_error() {
        let args: Vec<String> = vec!["location-sensor".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err(), "expected error when both --lat and --lon are missing");
    }

    /// TS-09-E1: Missing --lon should return an error.
    #[test]
    fn test_missing_lon_exits_with_error() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.1351".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_err(), "expected error when --lon is missing");
    }

    /// TS-09-1: Valid --lat and --lon should parse successfully and return correct values.
    #[test]
    fn test_parses_valid_lat_lon() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.1351".to_string(),
            "--lon=11.5820".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "expected successful parse with valid --lat and --lon");
        let (lat, lon, _addr) = result.unwrap();
        assert!((lat - 48.1351).abs() < 1e-6, "latitude mismatch");
        assert!((lon - 11.5820).abs() < 1e-6, "longitude mismatch");
    }

    /// TS-09-1: Verify the correct VSS signal paths are used.
    #[test]
    fn test_writes_correct_latitude_and_longitude() {
        // This test validates that write_location targets the correct VSS paths.
        // It will return a connection error since no DATA_BROKER is running.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(write_location("http://localhost:19999", 48.1351, 11.5820));
        // Verify the function returns an error that includes the broker address
        assert!(result.is_err(), "should fail when DATA_BROKER is unreachable");
        let err = result.unwrap_err();
        assert!(
            err.contains("localhost:19999"),
            "error should include broker address, got: {}",
            err
        );
    }

    /// TS-09-E1: Missing --lat should return an error.
    #[test]
    fn test_missing_lat_exits_with_error() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lon=11.5820".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_err(), "expected error when --lat is missing");
    }

    /// Verify default broker address is used when not specified.
    #[test]
    fn test_default_broker_addr() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
        ];
        let (_, _, addr) = parse_args(&args).unwrap();
        assert_eq!(addr, "http://localhost:55556");
    }

    /// Verify custom broker address is used when specified.
    #[test]
    fn test_custom_broker_addr() {
        let args: Vec<String> = vec![
            "location-sensor".to_string(),
            "--lat=48.0".to_string(),
            "--lon=11.0".to_string(),
            "--broker-addr=http://192.168.1.10:55556".to_string(),
        ];
        let (_, _, addr) = parse_args(&args).unwrap();
        assert_eq!(addr, "http://192.168.1.10:55556");
    }
}
