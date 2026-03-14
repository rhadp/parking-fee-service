//! Configuration module: reads env vars with defaults.
//!
//! Requirement: 08-REQ-7.1, 08-REQ-7.2

/// Default PARKING_OPERATOR URL (08-REQ-7.2).
pub const DEFAULT_OPERATOR_URL: &str = "http://localhost:8080";
/// Default DATA_BROKER gRPC address (08-REQ-7.2).
pub const DEFAULT_DATA_BROKER_ADDR: &str = "http://localhost:55556";
/// Default gRPC server port (08-REQ-7.2).
pub const DEFAULT_GRPC_PORT: u16 = 50053;
/// Default vehicle ID (08-REQ-7.2).
pub const DEFAULT_VEHICLE_ID: &str = "DEMO-VIN-001";
/// Default parking zone ID (08-REQ-7.2).
pub const DEFAULT_ZONE_ID: &str = "zone-demo-1";

/// Adaptor configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// URL of the PARKING_OPERATOR REST API.
    pub parking_operator_url: String,
    /// gRPC address of DATA_BROKER.
    pub data_broker_addr: String,
    /// Port on which this service listens for gRPC connections.
    pub grpc_port: u16,
    /// Vehicle identifier sent to the PARKING_OPERATOR.
    pub vehicle_id: String,
    /// Parking zone identifier sent to the PARKING_OPERATOR.
    pub zone_id: String,
}

/// Load configuration from environment variables, falling back to defaults.
///
/// Reads the following environment variables:
/// - `PARKING_OPERATOR_URL` → [`DEFAULT_OPERATOR_URL`]
/// - `DATA_BROKER_ADDR` → [`DEFAULT_DATA_BROKER_ADDR`]
/// - `GRPC_PORT` → [`DEFAULT_GRPC_PORT`] (parsed as u16, falls back to default on parse error)
/// - `VEHICLE_ID` → [`DEFAULT_VEHICLE_ID`]
/// - `ZONE_ID` → [`DEFAULT_ZONE_ID`]
pub fn load_config() -> Config {
    let grpc_port = std::env::var("GRPC_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_GRPC_PORT);

    Config {
        parking_operator_url: std::env::var("PARKING_OPERATOR_URL")
            .unwrap_or_else(|_| DEFAULT_OPERATOR_URL.to_string()),
        data_broker_addr: std::env::var("DATA_BROKER_ADDR")
            .unwrap_or_else(|_| DEFAULT_DATA_BROKER_ADDR.to_string()),
        grpc_port,
        vehicle_id: std::env::var("VEHICLE_ID")
            .unwrap_or_else(|_| DEFAULT_VEHICLE_ID.to_string()),
        zone_id: std::env::var("ZONE_ID")
            .unwrap_or_else(|_| DEFAULT_ZONE_ID.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mutex to serialise env-var mutations across parallel test threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // Helper: clear all adaptor env vars.
    fn clear_env() {
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");
    }

    // -----------------------------------------------------------------------
    // TS-08-15: Config reads from environment variables.
    // -----------------------------------------------------------------------

    /// TS-08-15: Config reflects env values when set.
    #[test]
    fn test_config_from_env_vars() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env();

        std::env::set_var("PARKING_OPERATOR_URL", "http://op:9090");
        std::env::set_var("DATA_BROKER_ADDR", "http://db:55556");
        std::env::set_var("GRPC_PORT", "50099");
        std::env::set_var("VEHICLE_ID", "TEST-VIN-42");
        std::env::set_var("ZONE_ID", "zone-test-7");

        let cfg = load_config();

        assert_eq!(cfg.parking_operator_url, "http://op:9090");
        assert_eq!(cfg.data_broker_addr, "http://db:55556");
        assert_eq!(cfg.grpc_port, 50099);
        assert_eq!(cfg.vehicle_id, "TEST-VIN-42");
        assert_eq!(cfg.zone_id, "zone-test-7");

        clear_env();
    }

    // -----------------------------------------------------------------------
    // TS-08-16: Missing env vars use defaults.
    // -----------------------------------------------------------------------

    /// TS-08-16: All defaults are applied when no env vars are set.
    #[test]
    fn test_config_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env();

        let cfg = load_config();

        assert_eq!(cfg.parking_operator_url, DEFAULT_OPERATOR_URL);
        assert_eq!(cfg.data_broker_addr, DEFAULT_DATA_BROKER_ADDR);
        assert_eq!(cfg.grpc_port, DEFAULT_GRPC_PORT);
        assert_eq!(cfg.vehicle_id, DEFAULT_VEHICLE_ID);
        assert_eq!(cfg.zone_id, DEFAULT_ZONE_ID);
    }
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_env() {
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");
    }

    // -----------------------------------------------------------------------
    // TS-08-P6: Config defaults — any subset of env vars
    // -----------------------------------------------------------------------

    proptest! {
        /// TS-08-P6: For any subset of env vars set, unset ones use defaults.
        #[test]
        fn proptest_config_defaults(
            set_operator_url in proptest::bool::ANY,
            set_broker_addr in proptest::bool::ANY,
            set_grpc_port in proptest::bool::ANY,
            set_vehicle_id in proptest::bool::ANY,
            set_zone_id in proptest::bool::ANY,
            operator_url in "http://[a-z]{3,8}:[0-9]{4,5}",
            grpc_port in 1024u16..=65535,
        ) {
            let _guard = ENV_LOCK.lock().unwrap();
            clear_env();

            if set_operator_url {
                std::env::set_var("PARKING_OPERATOR_URL", &operator_url);
            }
            if set_broker_addr {
                std::env::set_var("DATA_BROKER_ADDR", "http://db:55556");
            }
            if set_grpc_port {
                std::env::set_var("GRPC_PORT", grpc_port.to_string());
            }
            if set_vehicle_id {
                std::env::set_var("VEHICLE_ID", "PROP-VIN");
            }
            if set_zone_id {
                std::env::set_var("ZONE_ID", "prop-zone");
            }

            let cfg = load_config();

            if set_operator_url {
                prop_assert_eq!(&cfg.parking_operator_url, &operator_url);
            } else {
                prop_assert_eq!(&cfg.parking_operator_url, DEFAULT_OPERATOR_URL);
            }
            if set_broker_addr {
                prop_assert_eq!(&cfg.data_broker_addr, "http://db:55556");
            } else {
                prop_assert_eq!(&cfg.data_broker_addr, DEFAULT_DATA_BROKER_ADDR);
            }
            if set_grpc_port {
                prop_assert_eq!(cfg.grpc_port, grpc_port);
            } else {
                prop_assert_eq!(cfg.grpc_port, DEFAULT_GRPC_PORT);
            }
            if set_vehicle_id {
                prop_assert_eq!(&cfg.vehicle_id, "PROP-VIN");
            } else {
                prop_assert_eq!(&cfg.vehicle_id, DEFAULT_VEHICLE_ID);
            }
            if set_zone_id {
                prop_assert_eq!(&cfg.zone_id, "prop-zone");
            } else {
                prop_assert_eq!(&cfg.zone_id, DEFAULT_ZONE_ID);
            }

            clear_env();
        }
    }
}
