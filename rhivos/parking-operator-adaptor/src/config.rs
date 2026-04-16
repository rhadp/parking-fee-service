//! Service configuration — reads env vars with defaults.

/// Default PARKING_OPERATOR REST base URL.
pub const DEFAULT_PARKING_OPERATOR_URL: &str = "http://localhost:8080";
/// Default DATA_BROKER gRPC address.
pub const DEFAULT_DATA_BROKER_ADDR: &str = "http://localhost:55556";
/// Default gRPC listen port.
pub const DEFAULT_GRPC_PORT: u16 = 50053;
/// Default vehicle identifier.
pub const DEFAULT_VEHICLE_ID: &str = "DEMO-VIN-001";
/// Default parking zone identifier.
pub const DEFAULT_ZONE_ID: &str = "zone-demo-1";

/// Configuration error.
#[derive(Debug)]
pub enum ConfigError {
    /// GRPC_PORT contained a non-numeric or out-of-range value.
    InvalidPort(String),
}

/// Parsed service configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub parking_operator_url: String,
    pub data_broker_addr: String,
    pub grpc_port: u16,
    pub vehicle_id: String,
    pub zone_id: String,
}

/// Load configuration from environment variables.
///
/// Returns `ConfigError::InvalidPort` when `GRPC_PORT` is not a valid u16.
pub fn load_config() -> Result<Config, ConfigError> {
    todo!("load_config not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-08-18: Configuration Defaults
    #[test]
    fn test_config_defaults() {
        // Remove all config env vars so defaults are used.
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");

        let config = load_config().expect("load_config should succeed with defaults");
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

        let result = load_config();

        // Clean up before asserting.
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");

        let config = result.expect("load_config should succeed with custom values");
        assert_eq!(config.parking_operator_url, "http://op.example.com:9090");
        assert_eq!(config.data_broker_addr, "http://10.0.0.5:55556");
        assert_eq!(config.grpc_port, 50099);
        assert_eq!(config.vehicle_id, "VIN-CUSTOM-123");
        assert_eq!(config.zone_id, "zone-custom-1");
    }

    // TS-08-E10: GRPC_PORT Non-Numeric
    #[test]
    fn test_config_invalid_grpc_port() {
        std::env::set_var("GRPC_PORT", "abc");
        let result = load_config();
        std::env::remove_var("GRPC_PORT");

        assert!(
            result.is_err(),
            "load_config should return Err for non-numeric GRPC_PORT"
        );
        assert!(
            matches!(result.unwrap_err(), ConfigError::InvalidPort(_)),
            "error should be InvalidPort variant"
        );
    }
}
