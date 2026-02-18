//! RHIVOS Locking Service.
//!
//! This service manages vehicle lock/unlock operations via the Kuksa Databroker.
//! It subscribes to lock commands, validates them against safety rules (speed
//! and door state), and writes the result back to the databroker.
//!
//! # Startup
//!
//! 1. Parse configuration from CLI args / environment variables.
//! 2. Connect to Kuksa Databroker (with exponential backoff on failure).
//! 3. Subscribe to `Vehicle.Command.Door.Lock`.
//! 4. For each command: validate, execute if safe, report result.
//!
//! # Shutdown
//!
//! The service shuts down gracefully on SIGINT or SIGTERM.
//!
//! # Requirements
//!
//! - 02-REQ-2.1: Connect and subscribe to lock commands via gRPC streaming
//! - 02-REQ-2.3: Accept databroker address via CLI / env var
//! - 02-REQ-2.E1: Retry connection with exponential backoff
//! - 02-REQ-2.E2: Re-subscribe on stream interruption

pub mod config;
pub mod lock_handler;
pub mod safety;

use config::Config;
use lock_handler::{KuksaDataBroker, run_lock_handler};
use parking_proto::kuksa_client::KuksaClient;

use clap::Parser;
use tokio::signal;
use tracing::{error, info, warn};

/// Maximum backoff delay between connection/subscription retries (in seconds).
const MAX_BACKOFF_SECS: u64 = 30;

/// Initial backoff delay (in seconds).
const INITIAL_BACKOFF_SECS: u64 = 1;

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

    // Run the handler loop with retry logic until shutdown.
    tokio::select! {
        result = run_with_retry(&config) => {
            if let Err(e) = result {
                error!(error = %e, "locking-service exited with error");
            }
        }
        _ = shutdown_signal() => {
            info!("locking-service shutting down");
        }
    }

    Ok(())
}

/// Wait for a shutdown signal (SIGINT or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm =
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
    }
}

/// Connect to the databroker and run the handler loop, retrying on failures.
///
/// Implements:
/// - 02-REQ-2.E1: Exponential backoff on connection failure
/// - 02-REQ-2.E2: Re-subscribe on stream interruption
async fn run_with_retry(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut backoff_secs = INITIAL_BACKOFF_SECS;

    loop {
        // Connect to the databroker with retry.
        let client = match connect_with_backoff(&config.databroker_addr, &mut backoff_secs).await {
            Some(c) => c,
            None => {
                // This only returns None if we should stop retrying,
                // which we never do — we retry indefinitely.
                continue;
            }
        };

        let broker = KuksaDataBroker::new(client.clone());

        // Run the handler. On success (stream ended cleanly) or error
        // (stream interrupted), re-subscribe.
        match run_lock_handler(&client, &broker, config).await {
            Ok(()) => {
                // Stream ended cleanly — re-subscribe.
                warn!("subscription stream ended, re-subscribing...");
            }
            Err(e) => {
                // 02-REQ-2.E2: re-subscribe on stream interruption.
                warn!(error = %e, "subscription interrupted, re-subscribing...");
            }
        }

        // Reset backoff on successful connection (but stream ended).
        backoff_secs = INITIAL_BACKOFF_SECS;
    }
}

/// Attempt to connect to the Kuksa Databroker with exponential backoff.
///
/// Returns the connected client. Retries indefinitely, logging each attempt.
async fn connect_with_backoff(
    addr: &str,
    backoff_secs: &mut u64,
) -> Option<KuksaClient> {
    loop {
        info!(addr, "connecting to Kuksa Databroker");
        match KuksaClient::connect(addr).await {
            Ok(client) => {
                info!(addr, "connected to Kuksa Databroker");
                return Some(client);
            }
            Err(e) => {
                warn!(
                    error = %e,
                    retry_in_secs = *backoff_secs,
                    "failed to connect to Kuksa Databroker, retrying..."
                );
                tokio::time::sleep(std::time::Duration::from_secs(*backoff_secs)).await;
                *backoff_secs = (*backoff_secs * 2).min(MAX_BACKOFF_SECS);
            }
        }
    }
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

    #[test]
    fn backoff_constants_are_sane() {
        assert!(INITIAL_BACKOFF_SECS > 0);
        assert!(MAX_BACKOFF_SECS >= INITIAL_BACKOFF_SECS);
    }
}
