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
        // Stub: will be implemented in task group 2
        todo!("Config::from_env not yet implemented")
    }
}
