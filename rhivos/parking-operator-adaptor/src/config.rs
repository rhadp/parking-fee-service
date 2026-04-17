//! Service configuration.
//!
//! Reads all configuration from environment variables with defaults per
//! 08-REQ-7.1 through 08-REQ-7.5.

#![allow(dead_code)]

/// Configuration for the PARKING_OPERATOR_ADAPTOR service.
#[derive(Debug, Clone)]
pub struct Config {
    /// PARKING_OPERATOR REST base URL (08-REQ-7.1).
    pub parking_operator_url: String,
    /// DATA_BROKER gRPC address (08-REQ-7.2).
    pub data_broker_addr: String,
    /// gRPC listen port (08-REQ-7.3).
    pub grpc_port: u16,
    /// Vehicle identifier for operator requests (08-REQ-7.4).
    pub vehicle_id: String,
    /// Default parking zone identifier (08-REQ-7.5).
    pub zone_id: String,
}

/// Error returned when configuration is invalid.
#[derive(Debug)]
pub enum ConfigError {
    /// GRPC_PORT contains a non-numeric value (08-REQ-7.E1).
    InvalidGrpcPort(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidGrpcPort(v) => {
                write!(f, "GRPC_PORT is not a valid port number: {v}")
            }
        }
    }
}

/// Load configuration from environment variables.
///
/// Returns [`ConfigError::InvalidGrpcPort`] if `GRPC_PORT` is set to a
/// non-numeric value (08-REQ-7.E1).
pub fn load_config() -> Result<Config, ConfigError> {
    let parking_operator_url = std::env::var("PARKING_OPERATOR_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let data_broker_addr = std::env::var("DATA_BROKER_ADDR")
        .unwrap_or_else(|_| "http://localhost:55556".to_string());

    let grpc_port = match std::env::var("GRPC_PORT") {
        Ok(val) => val
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidGrpcPort(val))?,
        Err(_) => 50053,
    };

    let vehicle_id =
        std::env::var("VEHICLE_ID").unwrap_or_else(|_| "DEMO-VIN-001".to_string());

    let zone_id =
        std::env::var("ZONE_ID").unwrap_or_else(|_| "zone-demo-1".to_string());

    Ok(Config {
        parking_operator_url,
        data_broker_addr,
        grpc_port,
        vehicle_id,
        zone_id,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mutex to serialize env-var mutations across parallel test threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// TS-08-18: Configuration defaults — all env vars unset.
    ///
    /// Requires: 08-REQ-7.1, 08-REQ-7.2, 08-REQ-7.3, 08-REQ-7.4, 08-REQ-7.5
    #[test]
    fn test_config_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Remove all relevant env vars so defaults apply.
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");

        let config = load_config().expect("default config must succeed");
        assert_eq!(
            config.parking_operator_url, "http://localhost:8080",
            "default PARKING_OPERATOR_URL"
        );
        assert_eq!(
            config.data_broker_addr, "http://localhost:55556",
            "default DATA_BROKER_ADDR"
        );
        assert_eq!(config.grpc_port, 50053u16, "default GRPC_PORT");
        assert_eq!(config.vehicle_id, "DEMO-VIN-001", "default VEHICLE_ID");
        assert_eq!(config.zone_id, "zone-demo-1", "default ZONE_ID");
    }

    /// TS-08-19: Configuration custom values — all env vars set.
    ///
    /// Requires: 08-REQ-7.1, 08-REQ-7.2, 08-REQ-7.3, 08-REQ-7.4, 08-REQ-7.5
    #[test]
    fn test_config_custom_values() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("PARKING_OPERATOR_URL", "http://op.example.com:9090");
        std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.5:55556");
        std::env::set_var("GRPC_PORT", "50099");
        std::env::set_var("VEHICLE_ID", "VIN-CUSTOM-123");
        std::env::set_var("ZONE_ID", "zone-custom-1");

        let config = load_config().expect("custom config must succeed");
        assert_eq!(config.parking_operator_url, "http://op.example.com:9090");
        assert_eq!(config.data_broker_addr, "http://10.0.0.5:55556");
        assert_eq!(config.grpc_port, 50099u16);
        assert_eq!(config.vehicle_id, "VIN-CUSTOM-123");
        assert_eq!(config.zone_id, "zone-custom-1");

        // Cleanup.
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");
    }

    /// TS-08-E10: Non-numeric GRPC_PORT causes error.
    ///
    /// Requires: 08-REQ-7.E1
    #[test]
    fn test_config_invalid_grpc_port() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("GRPC_PORT", "abc");
        let result = load_config();
        std::env::remove_var("GRPC_PORT");
        assert!(result.is_err(), "non-numeric GRPC_PORT must return an error");
    }
}
