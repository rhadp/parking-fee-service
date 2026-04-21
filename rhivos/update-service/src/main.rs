pub mod adapter;
pub mod config;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod service;
pub mod state;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] != "serve" {
        if args.iter().skip(1).any(|a| a.starts_with('-')) {
            eprintln!("usage: update-service serve");
            std::process::exit(0);
        }
        println!("usage: update-service serve");
        std::process::exit(0);
    }
    println!("update-service v{}", env!("CARGO_PKG_VERSION"));
    todo!("implemented in task group 5")
}
