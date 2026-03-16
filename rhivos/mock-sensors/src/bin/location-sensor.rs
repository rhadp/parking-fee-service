//! location-sensor — writes Vehicle.CurrentLocation.Latitude and .Longitude to DATA_BROKER.
//!
//! Usage:
//!   location-sensor --lat=<latitude> --lon=<longitude>
//!   location-sensor --help
//!
//! Requirements: 09-REQ-1.1, 09-REQ-1.E1, 09-REQ-1.E2, 09-REQ-5.1, 09-REQ-6.1

use mock_sensors::{get_broker_addr, BrokerWriter, SIGNAL_LATITUDE, SIGNAL_LONGITUDE};

fn print_usage() {
    eprintln!("location-sensor v{} - Write GPS location signals to DATA_BROKER", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    location-sensor --lat=<latitude> --lon=<longitude>");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --lat=<value>    Latitude in decimal degrees (required)");
    eprintln!("    --lon=<value>    Longitude in decimal degrees (required)");
    eprintln!("    --help           Print this help message");
    eprintln!();
    eprintln!("ENVIRONMENT:");
    eprintln!("    DATA_BROKER_ADDR    DATA_BROKER address [default: http://localhost:55556]");
    eprintln!();
    eprintln!("SIGNALS WRITTEN:");
    eprintln!("    {} (double)", SIGNAL_LATITUDE);
    eprintln!("    {} (double)", SIGNAL_LONGITUDE);
}

/// Parse --key=value style arguments from argv.
fn parse_flag(args: &[String], flag: &str) -> Option<String> {
    let prefix = format!("{}=", flag);
    args.iter().find_map(|a| {
        if a.starts_with(&prefix) {
            Some(a[prefix.len()..].to_string())
        } else {
            None
        }
    })
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Handle --help / -h or no args (01-REQ-4.1, 01-REQ-4.E1, 09-REQ-6.1).
    // Also handle unrecognized flags: exit 0 with usage.
    if args.len() == 1 || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        std::process::exit(0);
    }

    // Check for unrecognized flags (01-REQ-4.E1): any flag that isn't --lat=, --lon=, or --help.
    let has_unknown = args.iter().skip(1).any(|a| {
        a.starts_with('-') && !a.starts_with("--lat=") && !a.starts_with("--lon=") && a != "--help" && a != "-h"
    });
    if has_unknown {
        print_usage();
        std::process::exit(0);
    }

    // Parse --lat and --lon (09-REQ-1.1, 09-REQ-1.E1).
    let lat_str = match parse_flag(&args, "--lat") {
        Some(v) => v,
        None => {
            eprintln!("error: missing required argument --lat");
            eprintln!();
            print_usage();
            std::process::exit(0);
        }
    };

    let lon_str = match parse_flag(&args, "--lon") {
        Some(v) => v,
        None => {
            eprintln!("error: missing required argument --lon");
            eprintln!();
            print_usage();
            std::process::exit(0);
        }
    };

    let lat: f64 = match lat_str.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("error: invalid value for --lat: '{}'", lat_str);
            std::process::exit(1);
        }
    };

    let lon: f64 = match lon_str.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("error: invalid value for --lon: '{}'", lon_str);
            std::process::exit(1);
        }
    };

    // Connect to DATA_BROKER (09-REQ-1.E2, 09-REQ-5.1).
    let addr = get_broker_addr();
    let writer = match BrokerWriter::connect(&addr).await {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    // Write latitude signal (09-REQ-1.1).
    if let Err(e) = writer.set_double(SIGNAL_LATITUDE, lat).await {
        eprintln!("error: failed to write {}: {}", SIGNAL_LATITUDE, e);
        std::process::exit(1);
    }

    // Write longitude signal (09-REQ-1.1).
    if let Err(e) = writer.set_double(SIGNAL_LONGITUDE, lon).await {
        eprintln!("error: failed to write {}: {}", SIGNAL_LONGITUDE, e);
        std::process::exit(1);
    }

    // Success: exit 0 (09-REQ-1.1).
}
