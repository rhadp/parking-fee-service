use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};
use std::process;

/// RHIVOS door sensor mock.
///
/// Publishes door open/closed state to DATA_BROKER via kuksa.val.v1 gRPC Set RPC.
#[derive(Parser)]
#[command(name = "door-sensor")]
struct Cli {
    /// Set door to open (true)
    #[arg(long, group = "door_state")]
    open: bool,

    /// Set door to closed (false)
    #[arg(long, group = "door_state")]
    closed: bool,

    /// DATA_BROKER gRPC address
    #[arg(long, env = "DATABROKER_ADDR", default_value = "http://localhost:55556")]
    broker_addr: String,
}

#[tokio::main]
async fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    if !cli.open && !cli.closed {
        eprintln!("error: one of '--open' or '--closed' must be provided");
        process::exit(1);
    }

    let is_open = cli.open;

    if let Err(e) = publish_datapoint(
        &cli.broker_addr,
        "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
        DatapointValue::Bool(is_open),
    )
    .await
    {
        eprintln!("Error publishing door state: {e}");
        process::exit(1);
    }
}
