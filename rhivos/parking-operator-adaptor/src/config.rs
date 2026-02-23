//! Configuration for the PARKING_OPERATOR_ADAPTOR service.
//!
//! Configuration is loaded from environment variables with sensible defaults.

use std::env;

/// Configuration for the PARKING_OPERATOR_ADAPTOR.
#[derive(Debug, Clone)]
pub struct Config {
    /// gRPC listen address for the adaptor service.
    pub grpc_addr: String,
    /// DATA_BROKER gRPC address for cross-partition communication.
    pub databroker_addr: String,
    /// PARKING_OPERATOR REST base URL.
    pub operator_url: String,
    /// Vehicle identifier.
    pub vehicle_id: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            grpc_addr: "0.0.0.0:50052".to_string(),
            databroker_addr: "localhost:55556".to_string(),
            operator_url: "http://localhost:8090".to_string(),
            vehicle_id: "VIN12345".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// | Variable | Default | Description |
    /// |----------|---------|-------------|
    /// | `ADAPTOR_GRPC_ADDR` | `0.0.0.0:50052` | gRPC listen address |
    /// | `DATABROKER_ADDR` | `localhost:55556` | DATA_BROKER gRPC address |
    /// | `OPERATOR_URL` | `http://localhost:8090` | PARKING_OPERATOR REST base URL |
    /// | `VEHICLE_ID` | `VIN12345` | Vehicle identifier |
    pub fn from_env() -> Self {
        let defaults = Config::default();
        Config {
            grpc_addr: env::var("ADAPTOR_GRPC_ADDR").unwrap_or(defaults.grpc_addr),
            databroker_addr: env::var("DATABROKER_ADDR").unwrap_or(defaults.databroker_addr),
            operator_url: env::var("OPERATOR_URL").unwrap_or(defaults.operator_url),
            vehicle_id: env::var("VEHICLE_ID").unwrap_or(defaults.vehicle_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.grpc_addr, "0.0.0.0:50052");
        assert_eq!(config.databroker_addr, "localhost:55556");
        assert_eq!(config.operator_url, "http://localhost:8090");
        assert_eq!(config.vehicle_id, "VIN12345");
    }

    #[test]
    fn test_config_fields_are_clone() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(config.grpc_addr, cloned.grpc_addr);
        assert_eq!(config.databroker_addr, cloned.databroker_addr);
        assert_eq!(config.operator_url, cloned.operator_url);
        assert_eq!(config.vehicle_id, cloned.vehicle_id);
    }
}
