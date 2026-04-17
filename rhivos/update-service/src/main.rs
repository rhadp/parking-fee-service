// UPDATE_SERVICE — gRPC server for containerized adapter lifecycle management.
// TODO: full implementation with gRPC server, signal handling,
// config loading, broadcaster, state manager, podman executor, offload timer.

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!("update-service v0.1.0");
        println!("Usage: update-service serve");
        return;
    }

    // Reject unknown flags (01-REQ-4.E1).
    for arg in &args {
        if arg.starts_with('-') {
            eprintln!("Unknown flag: {arg}");
            eprintln!("Usage: update-service serve");
            process::exit(1);
        }
    }

    match args[0].as_str() {
        "serve" => {
            println!("update-service v0.1.0 — implementation pending");
        }
        unknown => {
            eprintln!("Unknown subcommand: {unknown}");
            eprintln!("Usage: update-service serve");
            process::exit(1);
        }
    }
}
