use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};

/// Publish mock GPS coordinates to DATA_BROKER.
#[derive(Parser)]
#[command(name = "location-sensor")]
struct Args {
    /// Latitude value (double)
    #[arg(long)]
    lat: f64,

    /// Longitude value (double)
    #[arg(long)]
    lon: f64,

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
        "Vehicle.CurrentLocation.Latitude",
        DatapointValue::Double(args.lat),
    )
    .await
    {
        eprintln!("Error publishing latitude: {e}");
        std::process::exit(1);
    }

    if let Err(e) = publish_datapoint(
        &args.broker_addr,
        "Vehicle.CurrentLocation.Longitude",
        DatapointValue::Double(args.lon),
    )
    .await
    {
        eprintln!("Error publishing longitude: {e}");
        std::process::exit(1);
    }
}
