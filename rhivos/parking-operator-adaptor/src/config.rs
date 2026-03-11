use std::env;

/// Configuration for the PARKING_OPERATOR_ADAPTOR.
/// Reads from environment variables with sensible defaults.
pub struct Config {
    pub parking_operator_url: String,
    pub data_broker_addr: String,
    pub grpc_port: u16,
    pub vehicle_id: String,
    pub zone_id: String,
}

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        Self {
            parking_operator_url: env::var("PARKING_OPERATOR_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            data_broker_addr: env::var("DATA_BROKER_ADDR")
                .unwrap_or_else(|_| "http://localhost:55556".to_string()),
            grpc_port: env::var("GRPC_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50052),
            vehicle_id: env::var("VEHICLE_ID")
                .unwrap_or_else(|_| "DEMO-VIN-001".to_string()),
            zone_id: env::var("ZONE_ID")
                .unwrap_or_else(|_| "zone-demo-1".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // Remove any env vars that might interfere
        env::remove_var("PARKING_OPERATOR_URL");
        env::remove_var("DATA_BROKER_ADDR");
        env::remove_var("GRPC_PORT");
        env::remove_var("VEHICLE_ID");
        env::remove_var("ZONE_ID");

        let config = Config::from_env();
        assert_eq!(config.parking_operator_url, "http://localhost:8080");
        assert_eq!(config.data_broker_addr, "http://localhost:55556");
        assert_eq!(config.grpc_port, 50052);
        assert_eq!(config.vehicle_id, "DEMO-VIN-001");
        assert_eq!(config.zone_id, "zone-demo-1");
    }
}
