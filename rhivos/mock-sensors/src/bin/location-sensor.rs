// location-sensor: publishes Vehicle.CurrentLocation.Latitude and .Longitude
// to DATA_BROKER via kuksa.val.v1 gRPC Set RPC.
//
// Requirements: 09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.E1, 09-REQ-1.E2

use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue, DEFAULT_BROKER_ADDR, LATITUDE_PATH, LONGITUDE_PATH};

#[derive(Parser)]
#[command(name = "location-sensor", about = "Publishes mock GPS coordinates to DATA_BROKER")]
struct Args {
    /// Latitude value (double) for Vehicle.CurrentLocation.Latitude
    #[arg(long)]
    lat: f64,

    /// Longitude value (double) for Vehicle.CurrentLocation.Longitude
    #[arg(long)]
    lon: f64,

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

    if let Err(e) = publish_datapoint(
        &args.broker_addr,
        LATITUDE_PATH,
        DatapointValue::Double(args.lat),
    )
    .await
    {
        eprintln!("Error publishing latitude: {e}");
        std::process::exit(1);
    }

    if let Err(e) = publish_datapoint(
        &args.broker_addr,
        LONGITUDE_PATH,
        DatapointValue::Double(args.lon),
    )
    .await
    {
        eprintln!("Error publishing longitude: {e}");
        std::process::exit(1);
    }
}
