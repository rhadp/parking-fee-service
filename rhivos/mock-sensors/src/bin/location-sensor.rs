//! location-sensor: publishes mock GPS coordinates to DATA_BROKER.
//!
//! Publishes `Vehicle.CurrentLocation.Latitude` and
//! `Vehicle.CurrentLocation.Longitude` (both as double) via the kuksa.val.v1
//! `Set` RPC and exits with code 0 on success or code 1 on any failure
//! (09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.E1, 09-REQ-1.E2).

use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue, DEFAULT_BROKER_ADDR, PATH_LATITUDE, PATH_LONGITUDE};

#[derive(Parser)]
#[command(name = "location-sensor", about = "Publish mock GPS coordinates to DATA_BROKER")]
struct Args {
    /// Latitude value (degrees, double precision).
    #[arg(long)]
    lat: f64,

    /// Longitude value (degrees, double precision).
    #[arg(long)]
    lon: f64,

    /// DATA_BROKER gRPC address.
    #[arg(long, env = "DATABROKER_ADDR", default_value = DEFAULT_BROKER_ADDR)]
    broker_addr: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(e) = publish_datapoint(&args.broker_addr, PATH_LATITUDE, DatapointValue::Double(args.lat)).await {
        eprintln!("Error publishing {}: {}", PATH_LATITUDE, e);
        std::process::exit(1);
    }

    if let Err(e) = publish_datapoint(&args.broker_addr, PATH_LONGITUDE, DatapointValue::Double(args.lon)).await {
        eprintln!("Error publishing {}: {}", PATH_LONGITUDE, e);
        std::process::exit(1);
    }
}
