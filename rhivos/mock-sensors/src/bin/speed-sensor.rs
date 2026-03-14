//! speed-sensor — writes Vehicle.Speed to DATA_BROKER.
//!
//! Usage:
//!   speed-sensor --speed=<value>
//!   speed-sensor --help
//!
//! Requirements: 09-REQ-1.2, 09-REQ-1.E1, 09-REQ-1.E2, 09-REQ-5.1, 09-REQ-6.1

use mock_sensors::{get_broker_addr, BrokerWriter, SIGNAL_SPEED};

fn print_usage() {
    eprintln!("speed-sensor v{} - Write vehicle speed signal to DATA_BROKER", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    speed-sensor --speed=<value>");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --speed=<value>    Speed in km/h (required)");
    eprintln!("    --help             Print this help message");
    eprintln!();
    eprintln!("ENVIRONMENT:");
    eprintln!("    DATA_BROKER_ADDR    DATA_BROKER address [default: http://localhost:55556]");
    eprintln!();
    eprintln!("SIGNALS WRITTEN:");
    eprintln!("    {} (float)", SIGNAL_SPEED);
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

    // Handle --help / -h (09-REQ-6.1).
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        std::process::exit(0);
    }

    // Parse --speed (09-REQ-1.2, 09-REQ-1.E1).
    let speed_str = match parse_flag(&args, "--speed") {
        Some(v) => v,
        None => {
            eprintln!("error: missing required argument --speed");
            eprintln!();
            print_usage();
            std::process::exit(1);
        }
    };

    let speed: f32 = match speed_str.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("error: invalid value for --speed: '{}'", speed_str);
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

    // Write Vehicle.Speed signal (09-REQ-1.2).
    if let Err(e) = writer.set_float(SIGNAL_SPEED, speed).await {
        eprintln!("error: failed to write {}: {}", SIGNAL_SPEED, e);
        std::process::exit(1);
    }

    // Success: exit 0 (09-REQ-1.2).
}
