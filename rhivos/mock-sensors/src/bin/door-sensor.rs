// door-sensor: publishes Vehicle.Cabin.Door.Row1.DriverSide.IsOpen to DATA_BROKER
// via kuksa.val.v1 gRPC Set RPC.
//
// Requirements: 09-REQ-3.1, 09-REQ-3.2, 09-REQ-3.E1, 09-REQ-3.E2

use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue, DEFAULT_BROKER_ADDR, DOOR_OPEN_PATH};

#[derive(Parser)]
#[command(name = "door-sensor", about = "Publishes mock door open/closed state to DATA_BROKER")]
struct Args {
    /// Set door state to open (IsOpen = true)
    #[arg(long, conflicts_with = "closed")]
    open: bool,

    /// Set door state to closed (IsOpen = false)
    #[arg(long, conflicts_with = "open")]
    closed: bool,

    /// DATA_BROKER gRPC address (flag > env > default)
    #[arg(long, env = "DATABROKER_ADDR", default_value = DEFAULT_BROKER_ADDR)]
    broker_addr: String,
}

#[tokio::main]
async fn main() {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let is_open = if args.open {
        true
    } else if args.closed {
        false
    } else {
        eprintln!("Error: one of --open or --closed is required");
        std::process::exit(1);
    };

    if let Err(e) =
        publish_datapoint(&args.broker_addr, DOOR_OPEN_PATH, DatapointValue::Bool(is_open)).await
    {
        eprintln!("Error publishing door state: {e}");
        std::process::exit(1);
    }
}
