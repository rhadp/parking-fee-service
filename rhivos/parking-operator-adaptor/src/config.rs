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
    /// Loads configuration from environment variables with defaults.
    pub fn from_env() -> Self {
        Self {
            parking_operator_url: std::env::var("PARKING_OPERATOR_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            data_broker_addr: std::env::var("DATA_BROKER_ADDR")
                .unwrap_or_else(|_| "http://localhost:55556".to_string()),
            grpc_port: std::env::var("GRPC_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50052),
            vehicle_id: std::env::var("VEHICLE_ID")
                .unwrap_or_else(|_| "DEMO-VIN-001".to_string()),
            zone_id: std::env::var("ZONE_ID")
                .unwrap_or_else(|_| "zone-demo-1".to_string()),
        }
    }
}
