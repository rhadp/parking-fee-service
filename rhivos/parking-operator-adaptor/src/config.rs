/// Configuration error type.
#[derive(Debug)]
pub struct ConfigError(pub String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "config error: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

/// Configuration parsed from environment variables.
///
/// All fields have documented defaults per 08-REQ-7.x.
#[derive(Debug, Clone)]
pub struct Config {
    /// PARKING_OPERATOR REST base URL (default: http://localhost:8080).
    pub parking_operator_url: String,
    /// DATA_BROKER gRPC address (default: http://localhost:55556).
    pub data_broker_addr: String,
    /// gRPC listen port (default: 50053).
    pub grpc_port: u16,
    /// Vehicle identifier (default: DEMO-VIN-001).
    pub vehicle_id: String,
    /// Default parking zone (default: zone-demo-1).
    pub zone_id: String,
}

/// Load configuration from environment variables.
///
/// Returns a `Config` with values from the environment, falling back to
/// defaults where the variable is not set. Returns `ConfigError` if
/// GRPC_PORT contains a non-numeric value.
pub fn load_config() -> Result<Config, ConfigError> {
    let parking_operator_url = std::env::var("PARKING_OPERATOR_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let data_broker_addr = std::env::var("DATA_BROKER_ADDR")
        .unwrap_or_else(|_| "http://localhost:55556".to_string());

    let grpc_port_str = std::env::var("GRPC_PORT").unwrap_or_else(|_| "50053".to_string());
    let grpc_port: u16 = grpc_port_str
        .parse()
        .map_err(|_| ConfigError(format!("GRPC_PORT is not a valid port number: {grpc_port_str}")))?;

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

    // TS-08-18: Verify all config env vars have correct defaults.
    #[test]
    #[serial]
    fn test_config_defaults() {
        // SAFETY: env var mutation is safe in single-threaded test context.
        unsafe {
            std::env::remove_var("PARKING_OPERATOR_URL");
            std::env::remove_var("DATA_BROKER_ADDR");
            std::env::remove_var("GRPC_PORT");
            std::env::remove_var("VEHICLE_ID");
            std::env::remove_var("ZONE_ID");
        }
        let config = load_config().expect("load_config should succeed with defaults");
        assert_eq!(config.parking_operator_url, "http://localhost:8080");
        assert_eq!(config.data_broker_addr, "http://localhost:55556");
        assert_eq!(config.grpc_port, 50053);
        assert_eq!(config.vehicle_id, "DEMO-VIN-001");
        assert_eq!(config.zone_id, "zone-demo-1");
    }

    // TS-08-19: Verify all config env vars are read from the environment.
    #[test]
    #[serial]
    fn test_config_custom_values() {
        // SAFETY: env var mutation is safe in single-threaded test context.
        unsafe {
            std::env::set_var("PARKING_OPERATOR_URL", "http://op.example.com:9090");
            std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.5:55556");
            std::env::set_var("GRPC_PORT", "50099");
            std::env::set_var("VEHICLE_ID", "VIN-CUSTOM-123");
            std::env::set_var("ZONE_ID", "zone-custom-1");
        }
        let config = load_config().expect("load_config should succeed with custom values");
        assert_eq!(config.parking_operator_url, "http://op.example.com:9090");
        assert_eq!(config.data_broker_addr, "http://10.0.0.5:55556");
        assert_eq!(config.grpc_port, 50099);
        assert_eq!(config.vehicle_id, "VIN-CUSTOM-123");
        assert_eq!(config.zone_id, "zone-custom-1");

        // Clean up.
        unsafe {
            std::env::remove_var("PARKING_OPERATOR_URL");
            std::env::remove_var("DATA_BROKER_ADDR");
            std::env::remove_var("GRPC_PORT");
            std::env::remove_var("VEHICLE_ID");
            std::env::remove_var("ZONE_ID");
        }
    }

    // TS-08-E10: Verify non-numeric GRPC_PORT causes error.
    #[test]
    #[serial]
    fn test_config_invalid_grpc_port() {
        // SAFETY: env var mutation is safe in single-threaded test context.
        unsafe {
            std::env::set_var("GRPC_PORT", "abc");
        }
        let result = load_config();
        assert!(result.is_err(), "load_config should fail for non-numeric GRPC_PORT");

        // Clean up.
        unsafe {
            std::env::remove_var("GRPC_PORT");
        }
    }

    // TS-08-1 Case 1: Verify default gRPC port is 50053 when GRPC_PORT not set.
    #[test]
    #[serial]
    fn test_grpc_port_default() {
        // SAFETY: env var mutation is safe in single-threaded test context.
        unsafe {
            std::env::remove_var("GRPC_PORT");
        }
        let config = load_config().expect("load_config should succeed");
        assert_eq!(config.grpc_port, 50053);
    }

    // TS-08-1 Case 2: Verify gRPC port is read from GRPC_PORT env var.
    #[test]
    #[serial]
    fn test_grpc_port_custom() {
        // SAFETY: env var mutation is safe in single-threaded test context.
        unsafe {
            std::env::set_var("GRPC_PORT", "50099");
        }
        let config = load_config().expect("load_config should succeed");
        assert_eq!(config.grpc_port, 50099);

        // Clean up.
        unsafe {
            std::env::remove_var("GRPC_PORT");
        }
    }
}
