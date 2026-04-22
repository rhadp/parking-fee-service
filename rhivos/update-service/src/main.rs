#![allow(dead_code)] // TG1: stubs are intentionally unused until implementation

pub mod adapter;
pub mod config;
pub mod grpc;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod state;

pub mod proto {
    tonic::include_proto!("update_service.v1");
}

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1].starts_with('-') {
        eprintln!("Usage: update-service");
        eprintln!("  RHIVOS update service skeleton.");
        process::exit(1);
    }
    println!("update-service v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
