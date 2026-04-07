pub mod command_validator;
pub mod config;
pub mod errors;
pub mod models;
pub mod nats_client;
pub mod telemetry;

#[cfg(test)]
mod tests;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        eprintln!("Usage: cloud-gateway-client");
        eprintln!("Error: unrecognized arguments: {:?}", &args[1..]);
        process::exit(1);
    }
    println!("cloud-gateway-client v0.1.0");
}

