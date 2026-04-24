use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};

/// Publish mock vehicle speed to DATA_BROKER.
#[derive(Parser)]
#[command(name = "speed-sensor")]
struct Args {
    /// Speed value (float)
    #[arg(long)]
    speed: f32,

    /// DATA_BROKER gRPC address
    #[arg(long, env = "DATABROKER_ADDR", default_value = "http://localhost:55556")]
    broker_addr: String,
}

#[tokio::main]
async fn main() {
    let args = Args::try_parse().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    if let Err(e) = publish_datapoint(
        &args.broker_addr,
        "Vehicle.Speed",
        DatapointValue::Float(args.speed),
    )
    .await
    {
        eprintln!("Error publishing speed: {e}");
        std::process::exit(1);
    }
}
