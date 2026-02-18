//! Configuration for the cloud gateway client.
//!
//! The [`Config`] struct holds all runtime configuration for the cloud gateway
//! client, parsed from command-line arguments and environment variables using
//! `clap`.
//!
//! # Defaults
//!
//! | Parameter | Default | Env Var |
//! |-----------|---------|---------|
//! | `mqtt_addr` | `localhost:1883` | `MQTT_ADDR` |
//! | `databroker_addr` | `http://localhost:55555` | `DATABROKER_ADDR` |
//! | `data_dir` | `./data` | `DATA_DIR` |
//! | `telemetry_interval` | `5` (seconds) | `TELEMETRY_INTERVAL` |
//!
//! # Requirements
//!
//! - 03-REQ-3.6: Accept MQTT and DATA_BROKER addresses via flags / env vars.
//! - 03-REQ-4.3: Accept telemetry interval via flag / env var.

use clap::Parser;

/// RHIVOS Cloud Gateway Client configuration.
///
/// Parsed from CLI arguments and environment variables.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "cloud-gateway-client",
    about = "RHIVOS cloud gateway client — bridges vehicle to cloud via MQTT"
)]
pub struct Config {
    /// MQTT broker address (host:port).
    ///
    /// Used to connect to Eclipse Mosquitto for vehicle-to-cloud communication.
    #[arg(long, env = "MQTT_ADDR", default_value = "localhost:1883")]
    pub mqtt_addr: String,

    /// Kuksa Databroker gRPC address.
    ///
    /// Must include the scheme (e.g., `http://localhost:55555`).
    #[arg(
        long,
        env = "DATABROKER_ADDR",
        default_value = "http://localhost:55555"
    )]
    pub databroker_addr: String,

    /// Directory for VIN and PIN persistence.
    ///
    /// On first startup, the generated VIN and pairing PIN are saved to
    /// `{data_dir}/vin.json`. On subsequent starts the persisted values are
    /// reused.
    #[arg(long, env = "DATA_DIR", default_value = "./data")]
    pub data_dir: String,

    /// Telemetry publishing interval in seconds.
    ///
    /// How often CLOUD_GATEWAY_CLIENT reads vehicle signals from DATA_BROKER
    /// and publishes a telemetry message to MQTT.
    #[arg(long, env = "TELEMETRY_INTERVAL", default_value = "5")]
    pub telemetry_interval: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = Config::parse_from(["cloud-gateway-client"]);
        assert_eq!(config.mqtt_addr, "localhost:1883");
        assert_eq!(config.databroker_addr, "http://localhost:55555");
        assert_eq!(config.data_dir, "./data");
        assert_eq!(config.telemetry_interval, 5);
    }

    #[test]
    fn custom_mqtt_addr() {
        let config = Config::parse_from([
            "cloud-gateway-client",
            "--mqtt-addr",
            "mosquitto:1883",
        ]);
        assert_eq!(config.mqtt_addr, "mosquitto:1883");
    }

    #[test]
    fn custom_databroker_addr() {
        let config = Config::parse_from([
            "cloud-gateway-client",
            "--databroker-addr",
            "http://kuksa:55555",
        ]);
        assert_eq!(config.databroker_addr, "http://kuksa:55555");
    }

    #[test]
    fn custom_data_dir() {
        let config = Config::parse_from([
            "cloud-gateway-client",
            "--data-dir",
            "/var/lib/cgc",
        ]);
        assert_eq!(config.data_dir, "/var/lib/cgc");
    }

    #[test]
    fn custom_telemetry_interval() {
        let config = Config::parse_from([
            "cloud-gateway-client",
            "--telemetry-interval",
            "10",
        ]);
        assert_eq!(config.telemetry_interval, 10);
    }

    #[test]
    fn all_custom_args() {
        let config = Config::parse_from([
            "cloud-gateway-client",
            "--mqtt-addr",
            "mqtt.example.com:1883",
            "--databroker-addr",
            "http://10.0.0.1:55555",
            "--data-dir",
            "/tmp/test-data",
            "--telemetry-interval",
            "30",
        ]);
        assert_eq!(config.mqtt_addr, "mqtt.example.com:1883");
        assert_eq!(config.databroker_addr, "http://10.0.0.1:55555");
        assert_eq!(config.data_dir, "/tmp/test-data");
        assert_eq!(config.telemetry_interval, 30);
    }

    #[test]
    fn config_is_clone() {
        let config = Config::parse_from(["cloud-gateway-client"]);
        let cloned = config.clone();
        assert_eq!(cloned.mqtt_addr, config.mqtt_addr);
        assert_eq!(cloned.databroker_addr, config.databroker_addr);
        assert_eq!(cloned.data_dir, config.data_dir);
        assert_eq!(cloned.telemetry_interval, config.telemetry_interval);
    }

    #[test]
    fn config_is_debug() {
        let config = Config::parse_from(["cloud-gateway-client"]);
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("mqtt_addr"));
        assert!(debug_str.contains("databroker_addr"));
        assert!(debug_str.contains("data_dir"));
        assert!(debug_str.contains("telemetry_interval"));
    }
}
