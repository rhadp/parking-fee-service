use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};

/// Mock door sensor: publishes door open/closed state to DATA_BROKER.
#[derive(Parser)]
#[command(name = "door-sensor", version = "0.1.0")]
struct Args {
    /// Set door state to open (true)
    #[arg(long, conflicts_with = "closed", required_unless_present = "closed")]
    open: bool,

    /// Set door state to closed (false)
    #[arg(long, conflicts_with = "open", required_unless_present = "open")]
    closed: bool,

    /// DATA_BROKER address
    #[arg(long, env = "DATABROKER_ADDR", default_value = "http://localhost:55556")]
    broker_addr: String,
}

#[tokio::main]
async fn main() {
    let args = match Args::try_parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let is_open = args.open;

    if let Err(e) = publish_datapoint(
        &args.broker_addr,
        "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
        DatapointValue::Bool(is_open),
    )
    .await
    {
        eprintln!("Error publishing IsOpen: {e}");
        std::process::exit(1);
    }
}
