//! Configuration for the parking operator adaptor.
//!
//! The [`Config`] struct holds all runtime configuration, parsed from
//! command-line arguments and environment variables using `clap`.
//!
//! # Defaults
//!
//! | Parameter | Default | Env Var |
//! |-----------|---------|---------|
//! | `listen_addr` | `0.0.0.0:50054` | `LISTEN_ADDR` |
//! | `databroker_addr` | `http://localhost:55555` | `DATABROKER_ADDR` |
//! | `parking_operator_url` | *(required)* | `PARKING_OPERATOR_URL` |
//! | `zone_id` | *(required)* | `ZONE_ID` |
//! | `vehicle_vin` | *(required)* | `VEHICLE_VIN` |
//!
//! # Requirements
//!
//! - 04-REQ-2.6: Accept configuration via environment variables.

use clap::Parser;

/// RHIVOS Parking Operator Adaptor configuration.
///
/// Parsed from CLI arguments and environment variables.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "parking-operator-adaptor",
    about = "RHIVOS parking operator adaptor — manages parking sessions via operator APIs"
)]
pub struct Config {
    /// Address to listen on for gRPC requests.
    #[arg(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:50054")]
    pub listen_addr: String,

    /// Kuksa Databroker gRPC address.
    ///
    /// Must include the scheme (e.g., `http://localhost:55555`).
    #[arg(
        long,
        env = "DATABROKER_ADDR",
        default_value = "http://localhost:55555"
    )]
    pub databroker_addr: String,

    /// Base URL of the PARKING_OPERATOR REST service.
    ///
    /// Example: `http://localhost:8082`
    #[arg(long, env = "PARKING_OPERATOR_URL")]
    pub parking_operator_url: String,

    /// Parking zone identifier.
    #[arg(long, env = "ZONE_ID")]
    pub zone_id: String,

    /// Vehicle identification number (VIN).
    #[arg(long, env = "VEHICLE_VIN")]
    pub vehicle_vin: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_with_required_fields() {
        let config = Config::parse_from([
            "parking-operator-adaptor",
            "--parking-operator-url",
            "http://localhost:8082",
            "--zone-id",
            "zone-1",
            "--vehicle-vin",
            "DEMO0000000000001",
        ]);
        assert_eq!(config.listen_addr, "0.0.0.0:50054");
        assert_eq!(config.databroker_addr, "http://localhost:55555");
        assert_eq!(config.parking_operator_url, "http://localhost:8082");
        assert_eq!(config.zone_id, "zone-1");
        assert_eq!(config.vehicle_vin, "DEMO0000000000001");
    }

    #[test]
    fn custom_listen_addr() {
        let config = Config::parse_from([
            "parking-operator-adaptor",
            "--listen-addr",
            "127.0.0.1:9999",
            "--parking-operator-url",
            "http://op:8082",
            "--zone-id",
            "z1",
            "--vehicle-vin",
            "VIN1",
        ]);
        assert_eq!(config.listen_addr, "127.0.0.1:9999");
    }

    #[test]
    fn custom_databroker_addr() {
        let config = Config::parse_from([
            "parking-operator-adaptor",
            "--databroker-addr",
            "http://kuksa:55555",
            "--parking-operator-url",
            "http://op:8082",
            "--zone-id",
            "z1",
            "--vehicle-vin",
            "VIN1",
        ]);
        assert_eq!(config.databroker_addr, "http://kuksa:55555");
    }

    #[test]
    fn all_custom_args() {
        let config = Config::parse_from([
            "parking-operator-adaptor",
            "--listen-addr",
            "10.0.0.1:50054",
            "--databroker-addr",
            "http://10.0.0.2:55555",
            "--parking-operator-url",
            "http://10.0.0.3:8082",
            "--zone-id",
            "zone-42",
            "--vehicle-vin",
            "WBA12345678901234",
        ]);
        assert_eq!(config.listen_addr, "10.0.0.1:50054");
        assert_eq!(config.databroker_addr, "http://10.0.0.2:55555");
        assert_eq!(config.parking_operator_url, "http://10.0.0.3:8082");
        assert_eq!(config.zone_id, "zone-42");
        assert_eq!(config.vehicle_vin, "WBA12345678901234");
    }

    #[test]
    fn config_is_clone() {
        let config = Config::parse_from([
            "parking-operator-adaptor",
            "--parking-operator-url",
            "http://op:8082",
            "--zone-id",
            "z1",
            "--vehicle-vin",
            "VIN1",
        ]);
        let cloned = config.clone();
        assert_eq!(cloned.listen_addr, config.listen_addr);
        assert_eq!(cloned.databroker_addr, config.databroker_addr);
        assert_eq!(cloned.parking_operator_url, config.parking_operator_url);
        assert_eq!(cloned.zone_id, config.zone_id);
        assert_eq!(cloned.vehicle_vin, config.vehicle_vin);
    }

    #[test]
    fn config_is_debug() {
        let config = Config::parse_from([
            "parking-operator-adaptor",
            "--parking-operator-url",
            "http://op:8082",
            "--zone-id",
            "z1",
            "--vehicle-vin",
            "VIN1",
        ]);
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("listen_addr"));
        assert!(debug_str.contains("databroker_addr"));
        assert!(debug_str.contains("parking_operator_url"));
        assert!(debug_str.contains("zone_id"));
        assert!(debug_str.contains("vehicle_vin"));
    }

    #[test]
    fn missing_required_field_errors() {
        // parking-operator-url is required; omitting it should fail
        let result = Config::try_parse_from(["parking-operator-adaptor"]);
        assert!(result.is_err());
    }
}
