//! door-sensor — writes Vehicle.Cabin.Door.Row1.DriverSide.IsOpen to DATA_BROKER.
//!
//! Usage:
//!   door-sensor --open
//!   door-sensor --closed
//!   door-sensor --help
//!
//! Requirements: 09-REQ-1.3, 09-REQ-1.E1, 09-REQ-1.E2, 09-REQ-5.1, 09-REQ-6.1

use mock_sensors::{get_broker_addr, BrokerWriter, SIGNAL_DOOR_OPEN};

fn print_usage() {
    eprintln!("door-sensor v{} - Write door open/closed state to DATA_BROKER", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    door-sensor --open");
    eprintln!("    door-sensor --closed");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --open     Set door state to open (IsOpen = true)");
    eprintln!("    --closed   Set door state to closed (IsOpen = false)");
    eprintln!("    --help     Print this help message");
    eprintln!();
    eprintln!("ENVIRONMENT:");
    eprintln!("    DATA_BROKER_ADDR    DATA_BROKER address [default: http://localhost:55556]");
    eprintln!();
    eprintln!("SIGNALS WRITTEN:");
    eprintln!("    {} (bool)", SIGNAL_DOOR_OPEN);
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Handle --help / -h or no args (01-REQ-4.1, 01-REQ-4.E1, 09-REQ-6.1).
    if args.len() == 1 || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        std::process::exit(0);
    }

    // Check for unrecognized flags (01-REQ-4.E1).
    let has_unknown = args.iter().skip(1).any(|a| {
        a.starts_with('-') && a != "--open" && a != "--closed" && a != "--help" && a != "-h"
    });
    if has_unknown {
        print_usage();
        std::process::exit(0);
    }

    // Determine door state from --open or --closed (09-REQ-1.3, 09-REQ-1.E1).
    let is_open: bool = if args.iter().any(|a| a == "--open") {
        true
    } else if args.iter().any(|a| a == "--closed") {
        false
    } else {
        eprintln!("error: must specify --open or --closed");
        eprintln!();
        print_usage();
        std::process::exit(0);
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

    // Write IsOpen signal (09-REQ-1.3).
    if let Err(e) = writer.set_bool(SIGNAL_DOOR_OPEN, is_open).await {
        eprintln!("error: failed to write {}: {}", SIGNAL_DOOR_OPEN, e);
        std::process::exit(1);
    }

    // Success: exit 0 (09-REQ-1.3).
}
