use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};
use std::process;

/// RHIVOS location sensor mock.
///
/// Publishes GPS coordinates to DATA_BROKER via kuksa.val.v1 gRPC Set RPC.
#[derive(Parser)]
#[command(name = "location-sensor")]
struct Cli {
    /// Latitude value (Vehicle.CurrentLocation.Latitude)
    #[arg(long)]
    lat: f64,

    /// Longitude value (Vehicle.CurrentLocation.Longitude)
    #[arg(long)]
    lon: f64,

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
        "Vehicle.CurrentLocation.Latitude",
        DatapointValue::Double(cli.lat),
    )
    .await
    {
        eprintln!("Error publishing latitude: {e}");
        process::exit(1);
    }

    if let Err(e) = publish_datapoint(
        &cli.broker_addr,
        "Vehicle.CurrentLocation.Longitude",
        DatapointValue::Double(cli.lon),
    )
    .await
    {
        eprintln!("Error publishing longitude: {e}");
        process::exit(1);
    }
}
