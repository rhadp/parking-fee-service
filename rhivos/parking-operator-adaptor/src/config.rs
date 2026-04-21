//! Configuration module for parking-operator-adaptor.
//!
//! Reads configuration from environment variables with sensible defaults.

/// Configuration for the parking-operator-adaptor service.
#[derive(Debug, Clone)]
pub struct Config {
    /// Base URL for the PARKING_OPERATOR REST API.
    pub parking_operator_url: String,
    /// gRPC address for DATA_BROKER (Kuksa Databroker).
    pub data_broker_addr: String,
    /// Port on which the gRPC server listens.
    pub grpc_port: u16,
    /// Vehicle identifier sent to the PARKING_OPERATOR.
    pub vehicle_id: String,
    /// Default parking zone identifier.
    pub zone_id: String,
}

/// Errors that can occur when loading configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// GRPC_PORT environment variable contained a non-numeric value.
    InvalidPort(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidPort(s) => write!(f, "invalid GRPC_PORT value: {s}"),
        }
    }
}

/// Load configuration from environment variables, using defaults where unset.
///
/// # Errors
/// Returns [`ConfigError::InvalidPort`] if `GRPC_PORT` is not a valid u16.
pub fn load_config() -> Result<Config, ConfigError> {
    let parking_operator_url = std::env::var("PARKING_OPERATOR_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let data_broker_addr = std::env::var("DATA_BROKER_ADDR")
        .unwrap_or_else(|_| "http://localhost:55556".to_string());

    let grpc_port = match std::env::var("GRPC_PORT") {
        Ok(val) => val.parse::<u16>().map_err(|_| ConfigError::InvalidPort(val))?,
        Err(_) => 50053,
    };

    let vehicle_id = std::env::var("VEHICLE_ID")
        .unwrap_or_else(|_| "DEMO-VIN-001".to_string());

    let zone_id = std::env::var("ZONE_ID")
        .unwrap_or_else(|_| "zone-demo-1".to_string());

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

    // Serialise env-var mutations across parallel test threads.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// TS-08-18 / TS-08-1: All env vars absent → defaults are used.
    ///
    /// Verifies: 08-REQ-7.1, 08-REQ-7.2, 08-REQ-7.3, 08-REQ-7.4, 08-REQ-7.5
    #[test]
    fn test_config_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");

        let config = load_config().unwrap();

        assert_eq!(config.parking_operator_url, "http://localhost:8080");
        assert_eq!(config.data_broker_addr, "http://localhost:55556");
        assert_eq!(config.grpc_port, 50053);
        assert_eq!(config.vehicle_id, "DEMO-VIN-001");
        assert_eq!(config.zone_id, "zone-demo-1");
    }

    /// TS-08-19: All env vars set → custom values are used.
    ///
    /// Verifies: 08-REQ-7.1, 08-REQ-7.2, 08-REQ-7.3, 08-REQ-7.4, 08-REQ-7.5
    #[test]
    fn test_config_custom_values() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("PARKING_OPERATOR_URL", "http://op.example.com:9090");
        std::env::set_var("DATA_BROKER_ADDR", "http://10.0.0.5:55556");
        std::env::set_var("GRPC_PORT", "50099");
        std::env::set_var("VEHICLE_ID", "VIN-CUSTOM-123");
        std::env::set_var("ZONE_ID", "zone-custom-1");

        let config = load_config().unwrap();

        assert_eq!(config.parking_operator_url, "http://op.example.com:9090");
        assert_eq!(config.data_broker_addr, "http://10.0.0.5:55556");
        assert_eq!(config.grpc_port, 50099);
        assert_eq!(config.vehicle_id, "VIN-CUSTOM-123");
        assert_eq!(config.zone_id, "zone-custom-1");

        std::env::remove_var("PARKING_OPERATOR_URL");
        std::env::remove_var("DATA_BROKER_ADDR");
        std::env::remove_var("GRPC_PORT");
        std::env::remove_var("VEHICLE_ID");
        std::env::remove_var("ZONE_ID");
    }

    /// TS-08-E10: Non-numeric GRPC_PORT → load_config returns Err.
    ///
    /// Verifies: 08-REQ-7.E1
    #[test]
    fn test_config_invalid_grpc_port() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("GRPC_PORT", "abc");

        let result = load_config();

        assert!(result.is_err(), "expected Err for invalid GRPC_PORT");
        assert!(
            matches!(result.unwrap_err(), ConfigError::InvalidPort(_)),
            "expected ConfigError::InvalidPort"
        );

        std::env::remove_var("GRPC_PORT");
    }
}
