//! speed-sensor — publishes mock vehicle speed to DATA_BROKER.
//!
//! Publishes `Vehicle.Speed` (float) via kuksa.val.v1 `Set` RPC,
//! then exits 0.  Exits 1 on argument errors or connection failures.
//!
//! Usage: speed-sensor --speed=<value> [--broker-addr=<addr>]

use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};

const VSS_SPEED: &str = "Vehicle.Speed";
const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";

#[derive(Parser)]
#[command(name = "speed-sensor", about = "Publish mock vehicle speed to DATA_BROKER")]
struct Args {
    /// Speed value to publish in km/h (float).
    #[arg(long)]
    speed: f32,

    /// DATA_BROKER address (overrides DATABROKER_ADDR env var).
    #[arg(long, env = "DATABROKER_ADDR", default_value = DEFAULT_BROKER_ADDR)]
    broker_addr: String,
}

fn main() {
    let args = Args::parse();

    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let result = runtime.block_on(async {
        publish_datapoint(
            &args.broker_addr,
            VSS_SPEED,
            DatapointValue::Float(args.speed),
        )
        .await
    });

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
