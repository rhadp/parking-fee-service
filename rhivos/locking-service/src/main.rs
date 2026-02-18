//! Locking Service skeleton.
//!
//! This service manages vehicle lock/unlock operations via the Kuksa Databroker.
//! In this skeleton, it starts a process that logs its listen address and waits
//! for a shutdown signal. No gRPC RPCs are registered because locking-service
//! acts as a Kuksa client, not a parking-proto gRPC server.

use clap::Parser;
use tokio::signal;
use tracing::{error, info};

/// RHIVOS Locking Service
#[derive(Parser, Debug)]
#[command(name = "locking-service", about = "RHIVOS locking service skeleton")]
struct Args {
    /// Address to listen on (for future use)
    #[arg(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:50051")]
    listen_addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // Validate the listen address parses correctly.
    let addr: std::net::SocketAddr = args.listen_addr.parse().map_err(|e| {
        error!("Invalid listen address '{}': {}", args.listen_addr, e);
        e
    })?;

    info!("locking-service starting on {}", addr);
    info!("locking-service is a Kuksa Databroker client — no gRPC server registered");
    info!("Waiting for shutdown signal (Ctrl+C)...");

    // Wait for shutdown signal.
    signal::ctrl_c().await?;
    info!("locking-service shutting down");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_default_args() {
        let args = Args::parse_from(["locking-service"]);
        assert_eq!(args.listen_addr, "0.0.0.0:50051");
    }

    #[test]
    fn cli_parses_custom_listen_addr() {
        let args = Args::parse_from(["locking-service", "--listen-addr", "127.0.0.1:9999"]);
        assert_eq!(args.listen_addr, "127.0.0.1:9999");
    }
}
