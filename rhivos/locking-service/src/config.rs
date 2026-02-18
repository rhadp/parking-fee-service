//! Configuration for the locking service.
//!
//! The [`Config`] struct holds all runtime configuration for the locking
//! service, parsed from command-line arguments and environment variables using
//! `clap`.
//!
//! # Defaults
//!
//! | Parameter | Default | Env Var |
//! |-----------|---------|---------|
//! | `databroker_addr` | `http://localhost:55555` | `DATABROKER_ADDR` |
//! | `max_speed_kmh` | `1.0` | — |
//!
//! # Requirements
//!
//! - 02-REQ-2.3: Accept DATA_BROKER address via `--databroker-addr` flag or
//!   `DATABROKER_ADDR` environment variable, defaulting to `localhost:55555`.

use clap::Parser;

/// RHIVOS Locking Service configuration.
///
/// Parsed from CLI arguments and environment variables.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "locking-service",
    about = "RHIVOS locking service — processes lock/unlock commands with safety validation"
)]
pub struct Config {
    /// Kuksa Databroker gRPC address.
    ///
    /// Must include the scheme (e.g., `http://localhost:55555`).
    #[arg(
        long,
        env = "DATABROKER_ADDR",
        default_value = "http://localhost:55555"
    )]
    pub databroker_addr: String,

    /// Maximum vehicle speed (km/h) at which lock/unlock commands are allowed.
    ///
    /// Commands are rejected when vehicle speed >= this threshold.
    #[arg(long, default_value = "1.0")]
    pub max_speed_kmh: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = Config::parse_from(["locking-service"]);
        assert_eq!(config.databroker_addr, "http://localhost:55555");
        assert!((config.max_speed_kmh - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn custom_databroker_addr() {
        let config = Config::parse_from([
            "locking-service",
            "--databroker-addr",
            "http://kuksa:55555",
        ]);
        assert_eq!(config.databroker_addr, "http://kuksa:55555");
    }

    #[test]
    fn custom_max_speed() {
        let config = Config::parse_from(["locking-service", "--max-speed-kmh", "5.0"]);
        assert!((config.max_speed_kmh - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn all_custom_args() {
        let config = Config::parse_from([
            "locking-service",
            "--databroker-addr",
            "http://10.0.0.1:55555",
            "--max-speed-kmh",
            "2.5",
        ]);
        assert_eq!(config.databroker_addr, "http://10.0.0.1:55555");
        assert!((config.max_speed_kmh - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn config_is_clone() {
        let config = Config::parse_from(["locking-service"]);
        let cloned = config.clone();
        assert_eq!(cloned.databroker_addr, config.databroker_addr);
        assert!((cloned.max_speed_kmh - config.max_speed_kmh).abs() < f32::EPSILON);
    }

    #[test]
    fn config_is_debug() {
        let config = Config::parse_from(["locking-service"]);
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("databroker_addr"));
        assert!(debug_str.contains("max_speed_kmh"));
    }
}
