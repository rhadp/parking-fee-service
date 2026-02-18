//! RHIVOS Locking Service.
//!
//! This service manages vehicle lock/unlock operations via the Kuksa Databroker.
//! It subscribes to lock commands, validates them against safety rules (speed
//! and door state), and writes the result back to the databroker.
//!
//! Currently a skeleton — the command handler loop will be added in task group 5.

pub mod config;
pub mod safety;

use config::Config;

use clap::Parser;
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    info!(
        "locking-service starting (databroker={})",
        config.databroker_addr
    );
    info!(
        "safety threshold: max_speed_kmh={}",
        config.max_speed_kmh
    );
    info!("locking-service is a Kuksa Databroker client — command handler not yet implemented");
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
    fn cli_parses_default_config() {
        let config = Config::parse_from(["locking-service"]);
        assert_eq!(config.databroker_addr, "http://localhost:55555");
        assert!((config.max_speed_kmh - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cli_parses_custom_databroker_addr() {
        let config = Config::parse_from([
            "locking-service",
            "--databroker-addr",
            "http://kuksa:55555",
        ]);
        assert_eq!(config.databroker_addr, "http://kuksa:55555");
    }
}
