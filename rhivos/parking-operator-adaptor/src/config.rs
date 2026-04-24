/// Configuration for the parking operator adaptor, parsed from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// PARKING_OPERATOR REST base URL.
    pub parking_operator_url: String,
    /// DATA_BROKER gRPC address.
    pub data_broker_addr: String,
    /// gRPC server listen port.
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
                write!(f, "invalid GRPC_PORT value: {val}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Load configuration from environment variables with defaults.
///
/// # Environment Variables
///
/// - `PARKING_OPERATOR_URL` — defaults to `http://localhost:8080`
/// - `DATA_BROKER_ADDR` — defaults to `http://localhost:55556`
/// - `GRPC_PORT` — defaults to `50053`
/// - `VEHICLE_ID` — defaults to `DEMO-VIN-001`
/// - `ZONE_ID` — defaults to `zone-demo-1`
///
/// # Errors
///
/// Returns `ConfigError::InvalidGrpcPort` if `GRPC_PORT` contains a
/// non-numeric value.
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
    use serial_test::serial;

    // TS-08-18: Configuration Defaults
    // Validates: [08-REQ-7.1], [08-REQ-7.2], [08-REQ-7.3], [08-REQ-7.4], [08-REQ-7.5]
    #[test]
    #[serial]
    fn test_config_defaults() {
        // GIVEN no env vars set
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");

        // WHEN load_config() is called
        let config = load_config().expect("load_config should succeed with defaults");

        // THEN all fields have their default values
        assert_eq!(config.parking_operator_url, "http://localhost:8080");
        assert_eq!(config.data_broker_addr, "http://localhost:55556");
        assert_eq!(config.grpc_port, 50053);
        assert_eq!(config.vehicle_id, "DEMO-VIN-001");
        assert_eq!(config.zone_id, "zone-demo-1");
    }

    // TS-08-19: Configuration Custom Values
    // Validates: [08-REQ-7.1], [08-REQ-7.2], [08-REQ-7.3], [08-REQ-7.4], [08-REQ-7.5]
    #[test]
    #[serial]
    fn test_config_custom_values() {
        // GIVEN custom env vars set
        std::env::set_var("PARKING_OPERATOR_URL", "http://op.example.com:9090");
        std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.5:55556");
        std::env::set_var("GRPC_PORT", "50099");
        std::env::set_var("VEHICLE_ID", "VIN-CUSTOM-123");
        std::env::set_var("ZONE_ID", "zone-custom-1");

        // WHEN load_config() is called
        let config = load_config().expect("load_config should succeed with custom values");

        // THEN all fields reflect custom values
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
    // Validates: [08-REQ-7.E1]
    #[test]
    #[serial]
    fn test_config_invalid_grpc_port() {
        // GIVEN GRPC_PORT is set to a non-numeric value
        std::env::set_var("GRPC_PORT", "abc");

        // WHEN load_config() is called
        let result = load_config();

        // THEN result is Err(ConfigError::InvalidGrpcPort)
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::InvalidGrpcPort(_)));

        // Clean up
        std::env::remove_var("GRPC_PORT");
    }
}
