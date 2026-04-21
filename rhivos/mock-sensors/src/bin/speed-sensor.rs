//! speed-sensor: publishes mock vehicle speed to DATA_BROKER.
//!
//! Publishes `Vehicle.Speed` (float) via the kuksa.val.v1 `Set` RPC and exits
//! with code 0 on success or code 1 on any failure
//! (09-REQ-2.1, 09-REQ-2.2, 09-REQ-2.E1, 09-REQ-2.E2).

use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue, DEFAULT_BROKER_ADDR, PATH_SPEED};

#[derive(Parser)]
#[command(name = "speed-sensor", about = "Publish mock vehicle speed to DATA_BROKER")]
struct Args {
    /// Speed value in km/h (float precision).
    #[arg(long)]
    speed: f32,

    /// DATA_BROKER gRPC address.
    #[arg(long, env = "DATABROKER_ADDR", default_value = DEFAULT_BROKER_ADDR)]
    broker_addr: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(e) = publish_datapoint(&args.broker_addr, PATH_SPEED, DatapointValue::Float(args.speed)).await {
        eprintln!("Error publishing {}: {}", PATH_SPEED, e);
        std::process::exit(1);
    }
}
