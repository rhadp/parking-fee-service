//! RHIVOS Cloud Gateway Client.
//!
//! This service bridges the vehicle to the cloud via MQTT (Eclipse Mosquitto).
//! It connects to the MQTT broker, registers the vehicle by publishing its VIN
//! and pairing PIN, subscribes to command and status request topics, and (in
//! later task groups) processes commands and publishes telemetry.
//!
//! # Startup sequence
//!
//! 1. Parse configuration from CLI args / environment variables.
//! 2. Load or generate VIN and pairing PIN (persisted to data directory).
//! 3. Connect to MQTT broker and subscribe to vehicle topics.
//! 4. Publish registration message to CLOUD_GATEWAY.
//! 5. Run the MQTT event loop until shutdown.
//!
//! # Shutdown
//!
//! The service shuts down gracefully on SIGINT or SIGTERM.
//!
//! # Requirements
//!
//! - 03-REQ-3.1: Subscribe to vehicle command and status topics.
//! - 03-REQ-3.6: Accept MQTT and DATA_BROKER addresses via CLI / env vars.
//! - 03-REQ-5.1: Generate VIN and PIN on first start, persist, log to stdout.
//! - 03-REQ-5.2: Publish registration message on every startup.
//! - 03-REQ-5.E3: Reuse persisted VIN and PIN on subsequent starts.

pub mod config;
pub mod messages;
pub mod mqtt;
pub mod vin;

use clap::Parser;
use std::path::Path;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = config::Config::parse();

    info!(
        mqtt_addr = %config.mqtt_addr,
        databroker_addr = %config.databroker_addr,
        data_dir = %config.data_dir,
        telemetry_interval_secs = config.telemetry_interval,
        "cloud-gateway-client starting"
    );

    // 1. Load or generate VIN and pairing PIN.
    let vin_data = vin::load_or_create(Path::new(&config.data_dir)).map_err(|e| {
        error!(error = %e, "failed to load/generate VIN");
        e
    })?;

    info!(
        vin = %vin_data.vin,
        pairing_pin = %vin_data.pairing_pin,
        "vehicle identity ready"
    );

    // 2. Connect to MQTT and publish registration.
    let (_mqtt_client, event_loop) =
        mqtt::connect_and_register(&config.mqtt_addr, &vin_data).await.map_err(|e| {
            error!(error = %e, "failed to connect to MQTT broker");
            e
        })?;

    info!("MQTT client connected and registration published");

    // 3. Run the MQTT event loop until shutdown.
    //    Command processing, result forwarding, and telemetry are wired in
    //    later task groups (6 and 7).
    tokio::select! {
        _ = mqtt::run_event_loop(event_loop) => {
            error!("MQTT event loop exited unexpectedly");
        }
        _ = shutdown_signal() => {
            info!("cloud-gateway-client shutting down");
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

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use clap::Parser;

    #[test]
    fn cli_parses_default_args() {
        let config = Config::parse_from(["cloud-gateway-client"]);
        assert_eq!(config.mqtt_addr, "localhost:1883");
        assert_eq!(config.databroker_addr, "http://localhost:55555");
        assert_eq!(config.data_dir, "./data");
        assert_eq!(config.telemetry_interval, 5);
    }

    #[test]
    fn cli_parses_custom_mqtt_addr() {
        let config = Config::parse_from([
            "cloud-gateway-client",
            "--mqtt-addr",
            "mosquitto:1883",
        ]);
        assert_eq!(config.mqtt_addr, "mosquitto:1883");
    }

    #[test]
    fn cli_parses_all_custom_args() {
        let config = Config::parse_from([
            "cloud-gateway-client",
            "--mqtt-addr",
            "mqtt:1883",
            "--databroker-addr",
            "http://kuksa:55555",
            "--data-dir",
            "/tmp/data",
            "--telemetry-interval",
            "10",
        ]);
        assert_eq!(config.mqtt_addr, "mqtt:1883");
        assert_eq!(config.databroker_addr, "http://kuksa:55555");
        assert_eq!(config.data_dir, "/tmp/data");
        assert_eq!(config.telemetry_interval, 10);
    }
}
