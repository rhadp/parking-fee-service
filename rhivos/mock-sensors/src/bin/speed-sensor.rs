use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};

/// Mock speed sensor: publishes vehicle speed to DATA_BROKER.
#[derive(Parser)]
#[command(name = "speed-sensor", version = "0.1.0")]
struct Args {
    /// Speed value (float)
    #[arg(long)]
    speed: f32,

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

    if let Err(e) = publish_datapoint(
        &args.broker_addr,
        "Vehicle.Speed",
        DatapointValue::Float(args.speed),
    )
    .await
    {
        eprintln!("Error publishing Speed: {e}");
        std::process::exit(1);
    }
}
