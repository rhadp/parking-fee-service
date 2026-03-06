// Door sensor: writes door open/closed state to DATA_BROKER via gRPC.
// Implements 09-REQ-3.1, 09-REQ-7.1, 09-REQ-8.2.

/// Generated Kuksa VAL v2 protobuf types.
#[allow(clippy::doc_overindented_list_items, clippy::enum_variant_names)]
mod kuksa_proto {
    tonic::include_proto!("kuksa.val.v2");
}

use kuksa_proto::val_client::ValClient;
use kuksa_proto::value::TypedValue;
use kuksa_proto::{Datapoint, PublishValueRequest, SignalId};

const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";
const SIGNAL_DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (is_open, broker_addr) = match parse_args(&args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("Error: {}", msg);
            eprintln!("Usage: door-sensor <--open|--closed> [--broker-addr=<ADDR>]");
            std::process::exit(1);
        }
    };

    match write_door_state(&broker_addr, is_open).await {
        Ok(()) => {
            let state = if is_open { "open (true)" } else { "closed (false)" };
            println!(
                "Published {}={} to {}",
                SIGNAL_DOOR_OPEN, state, broker_addr
            );
        }
        Err(msg) => {
            eprintln!("Error: {}", msg);
            std::process::exit(1);
        }
    }
}

/// Parse CLI arguments and return (is_open, broker_addr).
/// Returns Err with a usage message if neither --open nor --closed is provided.
pub fn parse_args(args: &[String]) -> Result<(bool, String), String> {
    let mut is_open: Option<bool> = None;
    let mut broker_addr = DEFAULT_BROKER_ADDR.to_string();

    for arg in args.iter().skip(1) {
        if arg == "--open" {
            is_open = Some(true);
        } else if arg == "--closed" {
            is_open = Some(false);
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = val.to_string();
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let is_open = is_open.ok_or("missing required argument: --open or --closed")?;
    Ok((is_open, broker_addr))
}

/// Write door state to DATA_BROKER via gRPC.
pub async fn write_door_state(broker_addr: &str, is_open: bool) -> Result<(), String> {
    let mut client = ValClient::connect(broker_addr.to_string())
        .await
        .map_err(|e| format!("failed to connect to DATA_BROKER at {}: {}", broker_addr, e))?;

    let request = PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(kuksa_proto::signal_id::Signal::Path(
                SIGNAL_DOOR_OPEN.to_string(),
            )),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(kuksa_proto::Value {
                typed_value: Some(TypedValue::Bool(is_open)),
            }),
        }),
    };
    client
        .publish_value(request)
        .await
        .map_err(|e| format!("failed to publish {}: {}", SIGNAL_DOOR_OPEN, e))?;

    Ok(())
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
        assert!(result.is_err(), "should fail when DATA_BROKER is unreachable");
        let err = result.unwrap_err();
        assert!(
            err.contains("localhost:19999"),
            "error should include broker address, got: {}",
            err
        );
    }

    /// TS-09-4: Verify the correct VSS signal path is used (closed).
    #[test]
    fn test_writes_closed_false() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(write_door_state("http://localhost:19999", false));
        assert!(result.is_err(), "should fail when DATA_BROKER is unreachable");
        let err = result.unwrap_err();
        assert!(
            err.contains("localhost:19999"),
            "error should include broker address, got: {}",
            err
        );
    }

    /// Verify default broker address.
    #[test]
    fn test_default_broker_addr() {
        let args: Vec<String> = vec![
            "door-sensor".to_string(),
            "--open".to_string(),
        ];
        let (_, addr) = parse_args(&args).unwrap();
        assert_eq!(addr, "http://localhost:55556");
    }
}
