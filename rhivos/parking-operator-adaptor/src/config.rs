/// Configuration parsed from environment variables.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// PARKING_OPERATOR REST base URL.
    pub parking_operator_url: String,
    /// DATA_BROKER gRPC address.
    pub data_broker_addr: String,
    /// gRPC listen port.
    pub grpc_port: u16,
    /// Vehicle identifier for operator requests.
    pub vehicle_id: String,
    /// Default parking zone identifier.
    pub zone_id: String,
}

/// Error type for configuration loading.
#[derive(Debug)]
pub enum ConfigError {
    /// GRPC_PORT contains a non-numeric value.
    InvalidGrpcPort(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
/// Reads `PARKING_OPERATOR_URL`, `DATA_BROKER_ADDR`, `GRPC_PORT`,
/// `VEHICLE_ID`, and `ZONE_ID` from the environment with defaults.
///
/// Returns `Err(ConfigError::InvalidGrpcPort)` if `GRPC_PORT` is
/// set but not a valid `u16`.
pub fn load_config() -> Result<Config, ConfigError> {
    todo!("load_config not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn clear_config_env() {
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");
    }

    // TS-08-18: Configuration Defaults
    // Verify all config env vars have correct defaults when not set.
    #[test]
    #[serial]
    fn test_config_defaults() {
        clear_config_env();
        let config = load_config().expect("load_config should succeed with defaults");
        assert_eq!(config.parking_operator_url, "http://localhost:8080");
        assert_eq!(config.data_broker_addr, "http://localhost:55556");
        assert_eq!(config.grpc_port, 50053);
        assert_eq!(config.vehicle_id, "DEMO-VIN-001");
        assert_eq!(config.zone_id, "zone-demo-1");
    }

    // TS-08-19: Configuration Custom Values
    // Verify all config env vars are read from the environment.
    #[test]
    #[serial]
    fn test_config_custom_values() {
        std::env::set_var("PARKING_OPERATOR_URL", "http://op.example.com:9090");
        std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.5:55556");
        std::env::set_var("GRPC_PORT", "50099");
        std::env::set_var("VEHICLE_ID", "VIN-CUSTOM-123");
        std::env::set_var("ZONE_ID", "zone-custom-1");

        let config = load_config().expect("load_config should succeed with custom values");
        assert_eq!(config.parking_operator_url, "http://op.example.com:9090");
        assert_eq!(config.data_broker_addr, "http://10.0.0.5:55556");
        assert_eq!(config.grpc_port, 50099);
        assert_eq!(config.vehicle_id, "VIN-CUSTOM-123");
        assert_eq!(config.zone_id, "zone-custom-1");

        clear_config_env();
    }

    // TS-08-1: gRPC Server Starts on Configured Port (config part)
    // Case 1: GRPC_PORT not set → default 50053.
    #[test]
    #[serial]
    fn test_grpc_port_default() {
        clear_config_env();
        let config = load_config().expect("load_config should succeed");
        assert_eq!(config.grpc_port, 50053);
    }

    // TS-08-1: gRPC Server Starts on Configured Port (config part)
    // Case 2: GRPC_PORT=50099.
    #[test]
    #[serial]
    fn test_grpc_port_custom() {
        clear_config_env();
        std::env::set_var("GRPC_PORT", "50099");
        let config = load_config().expect("load_config should succeed");
        assert_eq!(config.grpc_port, 50099);
        clear_config_env();
    }

    // TS-08-E10: GRPC_PORT Non-Numeric
    // Verify non-numeric GRPC_PORT causes load_config to return an error.
    #[test]
    #[serial]
    fn test_config_invalid_grpc_port() {
        clear_config_env();
        std::env::set_var("GRPC_PORT", "abc");
        let result = load_config();
        assert!(result.is_err(), "load_config should fail for non-numeric GRPC_PORT");
        match result.unwrap_err() {
            ConfigError::InvalidGrpcPort(_) => {} // expected
        }
        clear_config_env();
    }
}
