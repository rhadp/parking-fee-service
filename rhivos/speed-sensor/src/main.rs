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

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (speed, broker_addr) = match parse_args(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Usage: speed-sensor --speed=<SPEED> [--broker-addr=<ADDR>]");
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

/// Parse CLI arguments and return (speed, broker_addr).
/// Returns Err with a usage message if required arguments are missing.
fn parse_args(args: &[String]) -> Result<(f32, String), String> {
    let mut speed: Option<f32> = None;
    let mut broker_addr = DEFAULT_BROKER_ADDR.to_string();

    // Skip the program name (args[0])
    for arg in args.iter().skip(1) {
        if let Some(val) = arg.strip_prefix("--speed=") {
            speed = Some(val.parse::<f32>().map_err(|e| format!("invalid --speed value: {}", e))?);
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = val.to_string();
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let speed = speed.ok_or_else(|| "required argument --speed is missing".to_string())?;

    Ok((speed, broker_addr))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=50.5".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse with valid args");
        let (speed, _addr) = result.unwrap();
        assert!((speed - 50.5).abs() < f32::EPSILON, "Speed mismatch");
    }

    /// TS-09-2: Default broker address should be http://localhost:55556.
    #[test]
    fn test_default_broker_addr() {
        let args: Vec<String> = vec![
            "speed-sensor".to_string(),
            "--speed=0.0".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_ok(), "Expected successful parse");
        let (_speed, addr) = result.unwrap();
        assert_eq!(addr, "http://localhost:55556");
    }
}
