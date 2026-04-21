//! door-sensor: publishes mock door open/closed state to DATA_BROKER.
//!
//! Publishes `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (bool) via the
//! kuksa.val.v1 `Set` RPC: `--open` sets `true`, `--closed` sets `false`.
//! Exits with code 0 on success or code 1 on any failure.
//! (09-REQ-3.1, 09-REQ-3.2, 09-REQ-3.E1, 09-REQ-3.E2).

use clap::{ArgGroup, Parser};
use mock_sensors::{publish_datapoint, DatapointValue, DEFAULT_BROKER_ADDR, PATH_DOOR_IS_OPEN};

#[derive(Parser)]
#[command(
    name = "door-sensor",
    about = "Publish mock door state to DATA_BROKER",
    group(
        ArgGroup::new("state")
            .required(true)
            .args(["open", "closed"])
    )
)]
struct Args {
    /// Set door state to open (IsOpen = true).
    #[arg(long, group = "state")]
    open: bool,

    /// Set door state to closed (IsOpen = false).
    #[arg(long, group = "state")]
    closed: bool,

    /// DATA_BROKER gRPC address.
    #[arg(long, env = "DATABROKER_ADDR", default_value = DEFAULT_BROKER_ADDR)]
    broker_addr: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // The ArgGroup guarantees exactly one of --open or --closed is set.
    let is_open = args.open;

    if let Err(e) = publish_datapoint(&args.broker_addr, PATH_DOOR_IS_OPEN, DatapointValue::Bool(is_open)).await {
        eprintln!("Error publishing {}: {}", PATH_DOOR_IS_OPEN, e);
        std::process::exit(1);
    }
}
