use std::fmt;

/// Service configuration loaded from environment variables.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub parking_operator_url: String,
    pub data_broker_addr: String,
    pub grpc_port: u16,
    pub vehicle_id: String,
    pub zone_id: String,
}

/// Errors that can occur when loading configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// GRPC_PORT contains a non-numeric value.
    InvalidGrpcPort(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidGrpcPort(val) => {
                write!(f, "GRPC_PORT is not a valid port number: {val}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Load configuration from environment variables.
///
/// Defaults:
/// - `PARKING_OPERATOR_URL` → `http://localhost:8080`
/// - `DATA_BROKER_ADDR` → `http://localhost:55556`
/// - `GRPC_PORT` → `50053`
/// - `VEHICLE_ID` → `DEMO-VIN-001`
/// - `ZONE_ID` → `zone-demo-1`
///
/// Returns `ConfigError::InvalidGrpcPort` if `GRPC_PORT` is non-numeric.
pub fn load_config() -> Result<Config, ConfigError> {
    let parking_operator_url = std::env::var("PARKING_OPERATOR_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let data_broker_addr = std::env::var("DATA_BROKER_ADDR")
        .unwrap_or_else(|_| "http://localhost:55556".to_string());

    let grpc_port_str = std::env::var("GRPC_PORT").unwrap_or_else(|_| "50053".to_string());
    let grpc_port: u16 = grpc_port_str
        .parse()
        .map_err(|_| ConfigError::InvalidGrpcPort(grpc_port_str))?;

    let vehicle_id =
        std::env::var("VEHICLE_ID").unwrap_or_else(|_| "DEMO-VIN-001".to_string());

    let zone_id = std::env::var("ZONE_ID").unwrap_or_else(|_| "zone-demo-1".to_string());

    Ok(Config {
        parking_operator_url,
        data_broker_addr,
        grpc_port,
        vehicle_id,
        zone_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-08-18: Configuration Defaults
    #[test]
    fn test_config_defaults() {
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");

        let config = load_config().expect("should load default config");
        assert_eq!(config.parking_operator_url, "http://localhost:8080");
        assert_eq!(config.data_broker_addr, "http://localhost:55556");
        assert_eq!(config.grpc_port, 50053);
        assert_eq!(config.vehicle_id, "DEMO-VIN-001");
        assert_eq!(config.zone_id, "zone-demo-1");
    }

    // TS-08-19: Configuration Custom Values
    #[test]
    fn test_config_custom_values() {
        std::env::set_var("PARKING_OPERATOR_URL", "http://op.example.com:9090");
        std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.5:55556");
        std::env::set_var("GRPC_PORT", "50099");
        std::env::set_var("VEHICLE_ID", "VIN-CUSTOM-123");
        std::env::set_var("ZONE_ID", "zone-custom-1");

        let config = load_config().expect("should load custom config");
        assert_eq!(config.parking_operator_url, "http://op.example.com:9090");
        assert_eq!(config.data_broker_addr, "http://10.0.0.5:55556");
        assert_eq!(config.grpc_port, 50099);
        assert_eq!(config.vehicle_id, "VIN-CUSTOM-123");
        assert_eq!(config.zone_id, "zone-custom-1");

        // Clean up
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");
    }

    // TS-08-E10: GRPC_PORT Non-Numeric
    #[test]
    fn test_config_invalid_grpc_port() {
        std::env::set_var("GRPC_PORT", "abc");
        let result = load_config();
        assert!(result.is_err(), "non-numeric GRPC_PORT should produce an error");
        std::env::remove_var("GRPC_PORT");
    }

    // TS-08-1: gRPC Server Starts on Configured Port (config portion)
    #[test]
    fn test_config_grpc_port_default() {
        std::env::remove_var("GRPC_PORT");
        let config = load_config().expect("should load config");
        assert_eq!(config.grpc_port, 50053);
    }

    // TS-08-1 Case 2: custom GRPC_PORT
    #[test]
    fn test_config_grpc_port_custom() {
        std::env::set_var("GRPC_PORT", "50099");
        let config = load_config().expect("should load config");
        assert_eq!(config.grpc_port, 50099);
        std::env::remove_var("GRPC_PORT");
    }
}
