//! SPEED_SENSOR — mock sensor CLI tool.
//!
//! Writes Vehicle.Speed to DATA_BROKER via gRPC.
//!
//! Requirements: 02-REQ-6.2, 02-REQ-6.4, 02-REQ-6.5

use clap::Parser;
use databroker_client::DataValue;
use mock_sensors::{resolve_endpoint, signals, write_signal};

/// Mock speed sensor — write Vehicle.Speed to DATA_BROKER.
#[derive(Parser, Debug)]
#[command(name = "speed-sensor")]
#[command(about = "Write a speed value (km/h) to DATA_BROKER")]
struct Args {
    /// Speed value in km/h (float).
    #[arg(long)]
    speed: f32,

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

    if let Err(e) = write_signal(&client, signals::SPEED, DataValue::Float(args.speed)).await {
        eprintln!("Error: failed to write Vehicle.Speed: {e}");
        std::process::exit(1);
    }

    println!(
        "OK: Vehicle.Speed = {} written to DATA_BROKER at {}",
        args.speed, endpoint
    );
}
