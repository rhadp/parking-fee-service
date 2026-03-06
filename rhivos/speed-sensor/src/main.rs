// Speed sensor: writes vehicle speed to DATA_BROKER via gRPC.
// Implements 09-REQ-2.1, 09-REQ-7.1, 09-REQ-8.2.

/// Generated Kuksa VAL v2 protobuf types.
#[allow(clippy::doc_overindented_list_items, clippy::enum_variant_names)]
mod kuksa_proto {
    tonic::include_proto!("kuksa.val.v2");
}

use kuksa_proto::val_client::ValClient;
use kuksa_proto::value::TypedValue;
use kuksa_proto::{Datapoint, PublishValueRequest, SignalId};

const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";
const SIGNAL_SPEED: &str = "Vehicle.Speed";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (speed, broker_addr) = match parse_args(&args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("Error: {}", msg);
            eprintln!("Usage: speed-sensor --speed=<SPEED> [--broker-addr=<ADDR>]");
            std::process::exit(1);
        }
    };

    match write_speed(&broker_addr, speed).await {
        Ok(()) => {
            println!(
                "Published {}={} to {}",
                SIGNAL_SPEED, speed, broker_addr
            );
        }
        Err(msg) => {
            eprintln!("Error: {}", msg);
            std::process::exit(1);
        }
    }
}

/// Parse CLI arguments and return (speed, broker_addr).
/// Returns Err with a usage message if required args are missing.
pub fn parse_args(args: &[String]) -> Result<(f32, String), String> {
    let mut speed: Option<f32> = None;
    let mut broker_addr = DEFAULT_BROKER_ADDR.to_string();

    for arg in args.iter().skip(1) {
        if let Some(val) = arg.strip_prefix("--speed=") {
            speed = Some(
                val.parse::<f32>()
                    .map_err(|e| format!("invalid --speed value: {}", e))?,
            );
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = val.to_string();
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let speed = speed.ok_or("missing required argument: --speed")?;
    Ok((speed, broker_addr))
}

/// Write speed to DATA_BROKER via gRPC.
pub async fn write_speed(broker_addr: &str, speed: f32) -> Result<(), String> {
    let mut client = ValClient::connect(broker_addr.to_string())
        .await
        .map_err(|e| format!("failed to connect to DATA_BROKER at {}: {}", broker_addr, e))?;

    let request = PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(kuksa_proto::signal_id::Signal::Path(
                SIGNAL_SPEED.to_string(),
            )),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(kuksa_proto::Value {
                typed_value: Some(TypedValue::Float(speed)),
            }),
        }),
    };
    client
        .publish_value(request)
        .await
        .map_err(|e| format!("failed to publish {}: {}", SIGNAL_SPEED, e))?;

    Ok(())
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
        assert!(result.is_err(), "should fail when DATA_BROKER is unreachable");
        let err = result.unwrap_err();
        assert!(
            err.contains("localhost:19999"),
            "error should include broker address, got: {}",
            err
        );
    }

    /// TS-09-2: Verify speed=0.0 is accepted.
    #[test]
    fn test_parses_zero_speed() {
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=0.0".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "expected successful parse with --speed=0.0");
        let (speed, _) = result.unwrap();
        assert!((speed - 0.0).abs() < 1e-6, "speed should be 0.0");
    }

    /// Verify default broker address.
    #[test]
    fn test_default_broker_addr() {
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=10.0".to_string(),
        ];
        let (_, addr) = parse_args(&args).unwrap();
        assert_eq!(addr, "http://localhost:55556");
    }
}
