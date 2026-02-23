//! LOCATION_SENSOR — mock sensor CLI tool.
//!
//! Writes Vehicle.CurrentLocation.Latitude and
//! Vehicle.CurrentLocation.Longitude to DATA_BROKER via gRPC.
//!
//! Requirements: 02-REQ-6.1, 02-REQ-6.4, 02-REQ-6.5

use clap::Parser;
use databroker_client::DataValue;
use mock_sensors::{resolve_endpoint, signals, write_signal};

/// Mock location sensor — write latitude and longitude to DATA_BROKER.
#[derive(Parser, Debug)]
#[command(name = "location-sensor")]
#[command(about = "Write latitude and longitude to DATA_BROKER")]
struct Args {
    /// Latitude value (double, degrees).
    #[arg(long)]
    lat: f64,

    /// Longitude value (double, degrees).
    #[arg(long)]
    lon: f64,

    /// DATA_BROKER endpoint (UDS or TCP).
    /// Overrides DATABROKER_ADDR and DATABROKER_UDS_PATH env vars.
    #[arg(long)]
    endpoint: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let endpoint = resolve_endpoint(args.endpoint.as_deref());
    let client = match mock_sensors::connect(&endpoint).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: failed to connect to DATA_BROKER at {endpoint}: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = write_signal(
        &client,
        signals::LATITUDE,
        DataValue::Double(args.lat),
    )
    .await
    {
        eprintln!("Error: failed to write Vehicle.CurrentLocation.Latitude: {e}");
        std::process::exit(1);
    }

    if let Err(e) = write_signal(
        &client,
        signals::LONGITUDE,
        DataValue::Double(args.lon),
    )
    .await
    {
        eprintln!("Error: failed to write Vehicle.CurrentLocation.Longitude: {e}");
        std::process::exit(1);
    }

    println!(
        "OK: Latitude={}, Longitude={} written to DATA_BROKER at {}",
        args.lat, args.lon, endpoint
    );
}
