/// Default DATA_BROKER address.
pub const DEFAULT_DATABROKER_ADDR: &str = "http://localhost:55556";

/// Get the DATA_BROKER gRPC address from environment.
pub fn get_databroker_addr() -> String {
    todo!("get_databroker_addr not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-3: Configurable Databroker Address (default)
    #[test]
    fn test_databroker_addr_default() {
        // Remove env var to test default
        std::env::remove_var("DATABROKER_ADDR");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://localhost:55556");
    }

    // TS-03-3: Configurable Databroker Address (custom)
    #[test]
    fn test_databroker_addr_env() {
        std::env::set_var("DATABROKER_ADDR", "http://10.0.0.5:55556");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://10.0.0.5:55556");
        // Cleanup
        std::env::remove_var("DATABROKER_ADDR");
    }
}
