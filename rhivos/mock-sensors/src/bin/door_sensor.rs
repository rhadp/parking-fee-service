//! door-sensor — publishes mock door open/closed state to DATA_BROKER.
//!
//! Publishes `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (bool) via
//! kuksa.val.v1 `Set` RPC, then exits 0.  Exits 1 on argument errors
//! or connection failures.
//!
//! Usage: door-sensor (--open | --closed) [--broker-addr=<addr>]

use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};

const VSS_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";

#[derive(Parser)]
#[command(name = "door-sensor", about = "Publish mock door state to DATA_BROKER")]
struct Args {
    /// Set door to open state (IsOpen = true).
    #[arg(long)]
    open: bool,

    /// Set door to closed state (IsOpen = false).
    #[arg(long)]
    closed: bool,

    /// DATA_BROKER address (overrides DATABROKER_ADDR env var).
    #[arg(long, env = "DATABROKER_ADDR", default_value = DEFAULT_BROKER_ADDR)]
    broker_addr: String,
}

fn main() {
    let args = Args::parse();

    // Exactly one of --open or --closed must be provided.
    let is_open = match (args.open, args.closed) {
        (true, false) => true,
        (false, true) => false,
        (false, false) => {
            eprintln!("error: must provide --open or --closed");
            eprintln!("Usage: door-sensor (--open | --closed) [--broker-addr=<addr>]");
            std::process::exit(1);
        }
        (true, true) => {
            eprintln!("error: --open and --closed are mutually exclusive");
            std::process::exit(1);
        }
    };

    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let result = runtime.block_on(async {
        publish_datapoint(
            &args.broker_addr,
            VSS_IS_OPEN,
            DatapointValue::Bool(is_open),
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
