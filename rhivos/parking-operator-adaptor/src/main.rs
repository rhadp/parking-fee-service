//! PARKING_OPERATOR_ADAPTOR — bridges PARKING_APP (gRPC) with a PARKING_OPERATOR backend (REST)
//! and autonomously manages parking sessions based on lock/unlock events from DATA_BROKER.
//!
//! Usage: parking-operator-adaptor serve

pub mod broker;
pub mod config;
pub mod event_loop;
pub mod operator;
pub mod session;

#[cfg(test)]
pub mod proptest_cases;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_usage() {
    eprintln!(
        "Usage: parking-operator-adaptor serve

Environment:
  PARKING_OPERATOR_URL   Operator REST base URL (default: http://localhost:8080)
  DATA_BROKER_ADDR       DATA_BROKER gRPC address (default: http://localhost:55556)
  GRPC_PORT              gRPC listen port (default: 50053)
  VEHICLE_ID             Vehicle identifier (default: DEMO-VIN-001)
  ZONE_ID                Default parking zone (default: zone-demo-1)"
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    for arg in &args[1..] {
        if arg.starts_with('-') {
            print_usage();
            std::process::exit(1);
        }
    }

    match args.get(1).map(|s| s.as_str()) {
        Some("serve") => {
            let runtime =
                tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            let exit_code = runtime.block_on(run_service());
            std::process::exit(exit_code);
        }
        None => {
            println!("parking-operator-adaptor v{VERSION}");
        }
        Some(unknown) => {
            eprintln!("Unknown subcommand: {unknown}");
            print_usage();
            std::process::exit(1);
        }
    }
}

async fn run_service() -> i32 {
    // Implementation in task group 4.
    todo!("run_service not yet implemented")
}
