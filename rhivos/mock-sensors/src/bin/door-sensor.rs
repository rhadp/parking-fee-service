//! DOOR_SENSOR — mock sensor CLI tool.
//!
//! Writes Vehicle.Cabin.Door.Row1.DriverSide.IsOpen to DATA_BROKER
//! via gRPC.
//!
//! Requirements: 02-REQ-6.3, 02-REQ-6.4, 02-REQ-6.5

use std::str::FromStr;

use clap::Parser;
use databroker_client::DataValue;
use mock_sensors::{resolve_endpoint, signals, write_signal};

/// A boolean value that must be explicitly specified as "true" or "false".
#[derive(Debug, Clone, Copy)]
struct BoolArg(bool);

impl FromStr for BoolArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "true" | "1" => Ok(BoolArg(true)),
            "false" | "0" => Ok(BoolArg(false)),
            _ => Err(format!(
                "invalid value '{s}': expected 'true' or 'false'"
            )),
        }
    }
}

/// Mock door sensor — write door open/closed state to DATA_BROKER.
#[derive(Parser, Debug)]
#[command(name = "door-sensor")]
#[command(about = "Write door open/closed state to DATA_BROKER")]
struct Args {
    /// Door open state (true = open, false = closed).
    #[arg(long)]
    open: BoolArg,

    /// DATA_BROKER endpoint (UDS or TCP).
    /// Overrides DATABROKER_ADDR and DATABROKER_UDS_PATH env vars.
    #[arg(long)]
    endpoint: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let endpoint = resolve_endpoint(args.endpoint.as_deref());
    let client = match mock_sensors::connect(&endpoint).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: failed to connect to DATA_BROKER at {endpoint}: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = write_signal(
        &client,
        signals::DOOR_IS_OPEN,
        DataValue::Bool(args.open.0),
    )
    .await
    {
        eprintln!(
            "Error: failed to write Vehicle.Cabin.Door.Row1.DriverSide.IsOpen: {e}"
        );
        std::process::exit(1);
    }

    println!(
        "OK: IsOpen={} written to DATA_BROKER at {}",
        args.open.0, endpoint
    );
}
