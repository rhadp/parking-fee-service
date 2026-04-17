//! location-sensor — publishes mock GPS coordinates to DATA_BROKER.
//!
//! Publishes `Vehicle.CurrentLocation.Latitude` (double) and
//! `Vehicle.CurrentLocation.Longitude` (double) via kuksa.val.v1 `Set` RPC,
//! then exits 0.  Exits 1 on argument errors or connection failures.
//!
//! Usage: location-sensor --lat=<value> --lon=<value> [--broker-addr=<addr>]

use clap::Parser;
use mock_sensors::{publish_datapoint, DatapointValue};

const VSS_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
const VSS_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
const DEFAULT_BROKER_ADDR: &str = "http://localhost:55556";

#[derive(Parser)]
#[command(name = "location-sensor", about = "Publish mock GPS coordinates to DATA_BROKER")]
struct Args {
    /// Latitude value to publish (double).
    #[arg(long)]
    lat: f64,

    /// Longitude value to publish (double).
    #[arg(long)]
    lon: f64,

    /// DATA_BROKER address (overrides DATABROKER_ADDR env var).
    #[arg(long, env = "DATABROKER_ADDR", default_value = DEFAULT_BROKER_ADDR)]
    broker_addr: String,
}

fn main() {
    let args = Args::parse();

    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let result = runtime.block_on(async {
        publish_datapoint(&args.broker_addr, VSS_LATITUDE, DatapointValue::Double(args.lat))
            .await?;
        publish_datapoint(&args.broker_addr, VSS_LONGITUDE, DatapointValue::Double(args.lon))
            .await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    });

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
