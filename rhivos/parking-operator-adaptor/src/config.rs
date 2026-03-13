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
                .unwrap_or(50053),
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
    use std::sync::Mutex;

    /// Global lock to serialize tests that modify environment variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_config_env() {
        env::remove_var("PARKING_OPERATOR_URL");
        env::remove_var("DATA_BROKER_ADDR");
        env::remove_var("GRPC_PORT");
        env::remove_var("VEHICLE_ID");
        env::remove_var("ZONE_ID");
    }

    /// TS-08-16: Missing env vars use defaults.
    #[test]
    fn test_config_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_config_env();

        let config = Config::from_env();
        assert_eq!(config.parking_operator_url, "http://localhost:8080");
        assert_eq!(config.data_broker_addr, "http://localhost:55556");
        assert_eq!(config.grpc_port, 50053);
        assert_eq!(config.vehicle_id, "DEMO-VIN-001");
        assert_eq!(config.zone_id, "zone-demo-1");
    }

    /// TS-08-15: Config reads from environment variables.
    #[test]
    fn test_config_from_env_vars() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_config_env();

        env::set_var("PARKING_OPERATOR_URL", "http://op:9090");
        env::set_var("DATA_BROKER_ADDR", "http://broker:55557");
        env::set_var("GRPC_PORT", "50099");
        env::set_var("VEHICLE_ID", "TEST-VIN-999");
        env::set_var("ZONE_ID", "zone-test-42");

        let config = Config::from_env();
        assert_eq!(config.parking_operator_url, "http://op:9090");
        assert_eq!(config.data_broker_addr, "http://broker:55557");
        assert_eq!(config.grpc_port, 50099);
        assert_eq!(config.vehicle_id, "TEST-VIN-999");
        assert_eq!(config.zone_id, "zone-test-42");

        clear_config_env();
    }

    /// TS-08-P6: Missing env vars always use defined defaults.
    /// For any subset of env vars being set, unset vars use defaults.
    #[test]
    fn proptest_config_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();

        // Test all 32 combinations of 5 boolean flags
        for mask in 0u8..32 {
            clear_config_env();

            let set_operator = mask & 1 != 0;
            let set_broker = mask & 2 != 0;
            let set_port = mask & 4 != 0;
            let set_vehicle = mask & 8 != 0;
            let set_zone = mask & 16 != 0;

            let custom_operator = "http://custom:1234";
            let custom_broker = "http://custom-broker:5678";
            let custom_port = "40000";
            let custom_vehicle = "CUSTOM-VIN";
            let custom_zone = "custom-zone";

            if set_operator { env::set_var("PARKING_OPERATOR_URL", custom_operator); }
            if set_broker { env::set_var("DATA_BROKER_ADDR", custom_broker); }
            if set_port { env::set_var("GRPC_PORT", custom_port); }
            if set_vehicle { env::set_var("VEHICLE_ID", custom_vehicle); }
            if set_zone { env::set_var("ZONE_ID", custom_zone); }

            let cfg = Config::from_env();

            if set_operator {
                assert_eq!(cfg.parking_operator_url, custom_operator, "mask={mask}");
            } else {
                assert_eq!(cfg.parking_operator_url, "http://localhost:8080", "mask={mask}");
            }
            if set_broker {
                assert_eq!(cfg.data_broker_addr, custom_broker, "mask={mask}");
            } else {
                assert_eq!(cfg.data_broker_addr, "http://localhost:55556", "mask={mask}");
            }
            if set_port {
                assert_eq!(cfg.grpc_port, 40000u16, "mask={mask}");
            } else {
                assert_eq!(cfg.grpc_port, 50053u16, "mask={mask}");
            }
            if set_vehicle {
                assert_eq!(cfg.vehicle_id, custom_vehicle, "mask={mask}");
            } else {
                assert_eq!(cfg.vehicle_id, "DEMO-VIN-001", "mask={mask}");
            }
            if set_zone {
                assert_eq!(cfg.zone_id, custom_zone, "mask={mask}");
            } else {
                assert_eq!(cfg.zone_id, "zone-demo-1", "mask={mask}");
            }
        }

        clear_config_env();
    }
}
