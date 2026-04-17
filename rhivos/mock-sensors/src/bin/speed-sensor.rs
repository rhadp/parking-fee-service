// speed-sensor: publishes Vehicle.Speed to DATA_BROKER via kuksa.val.v1 gRPC Set RPC.
//
// Requirements: 09-REQ-2.1, 09-REQ-2.2, 09-REQ-2.E1, 09-REQ-2.E2

use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue, DEFAULT_BROKER_ADDR, SPEED_PATH};

#[derive(Parser)]
#[command(name = "speed-sensor", about = "Publishes mock vehicle speed to DATA_BROKER")]
struct Args {
    /// Speed value (float) for Vehicle.Speed
    #[arg(long)]
    speed: f32,

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

    if let Err(e) =
        publish_datapoint(&args.broker_addr, SPEED_PATH, DatapointValue::Float(args.speed)).await
    {
        eprintln!("Error publishing speed: {e}");
        std::process::exit(1);
    }
}
