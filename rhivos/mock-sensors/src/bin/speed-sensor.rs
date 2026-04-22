use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};
use std::process;

/// RHIVOS speed sensor mock.
///
/// Publishes vehicle speed to DATA_BROKER via kuksa.val.v1 gRPC Set RPC.
#[derive(Parser)]
#[command(name = "speed-sensor")]
struct Cli {
    /// Speed value in km/h (Vehicle.Speed)
    #[arg(long)]
    speed: f32,

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

    if let Err(e) = publish_datapoint(
        &cli.broker_addr,
        "Vehicle.Speed",
        DatapointValue::Float(cli.speed),
    )
    .await
    {
        eprintln!("Error publishing speed: {e}");
        process::exit(1);
    }
}
