/// Default DATA_BROKER address.
pub const DEFAULT_DATABROKER_ADDR: &str = "http://localhost:55556";

/// Get the DATA_BROKER gRPC address from environment.
pub fn get_databroker_addr() -> String {
    std::env::var("DATABROKER_ADDR").unwrap_or_else(|_| DEFAULT_DATABROKER_ADDR.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialise env-var mutations across parallel test threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // TS-03-3: Configurable Databroker Address (default)
    #[test]
    fn test_databroker_addr_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("DATABROKER_ADDR");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://localhost:55556");
    }

    // TS-03-3: Configurable Databroker Address (custom)
    #[test]
    fn test_databroker_addr_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("DATABROKER_ADDR", "http://10.0.0.5:55556");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://10.0.0.5:55556");
        std::env::remove_var("DATABROKER_ADDR");
    }
}
