//! Cloud Gateway Client skeleton.
//!
//! This service bridges the vehicle to the cloud via MQTT (Eclipse Mosquitto).
//! In this skeleton, it starts a process that logs its listen address and waits
//! for a shutdown signal. No gRPC RPCs are registered because cloud-gateway-client
//! acts as an MQTT client, not a parking-proto gRPC server.

// Message types are defined now but used by later task groups (MQTT client,
// command handler, telemetry publisher, etc.).
#[allow(dead_code)]
pub(crate) mod messages;

use clap::Parser;
use tokio::signal;
use tracing::{error, info};

/// RHIVOS Cloud Gateway Client
#[derive(Parser, Debug)]
#[command(
    name = "cloud-gateway-client",
    about = "RHIVOS cloud gateway client skeleton"
)]
struct Args {
    /// Address to listen on (for future use)
    #[arg(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:50052")]
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

    info!("cloud-gateway-client starting on {}", addr);
    info!("cloud-gateway-client is an MQTT client — no gRPC server registered");
    info!("Waiting for shutdown signal (Ctrl+C)...");

    // Wait for shutdown signal.
    signal::ctrl_c().await?;
    info!("cloud-gateway-client shutting down");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_default_args() {
        let args = Args::parse_from(["cloud-gateway-client"]);
        assert_eq!(args.listen_addr, "0.0.0.0:50052");
    }

    #[test]
    fn cli_parses_custom_listen_addr() {
        let args =
            Args::parse_from(["cloud-gateway-client", "--listen-addr", "127.0.0.1:9999"]);
        assert_eq!(args.listen_addr, "127.0.0.1:9999");
    }
}
