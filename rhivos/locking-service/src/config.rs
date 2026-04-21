//! Service configuration from environment variables.

/// Read the DATA_BROKER gRPC address from `DATABROKER_ADDR` env var.
/// Falls back to `http://localhost:55556` if not set.
pub fn get_databroker_addr() -> String {
    todo!("implemented in task group 2")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize env-var tests to prevent parallel interference.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // TS-03-3: Default address when DATABROKER_ADDR not set
    #[test]
    fn test_databroker_addr_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("DATABROKER_ADDR");
        assert_eq!(get_databroker_addr(), "http://localhost:55556");
    }

    // TS-03-3: Custom address from DATABROKER_ADDR env var
    #[test]
    fn test_databroker_addr_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("DATABROKER_ADDR", "http://10.0.0.5:55556");
        let addr = get_databroker_addr();
        std::env::remove_var("DATABROKER_ADDR");
        assert_eq!(addr, "http://10.0.0.5:55556");
    }
}
