/// Returns the DATA_BROKER gRPC address from the `DATABROKER_ADDR` environment
/// variable, falling back to `http://localhost:55556` if not set.
pub fn get_databroker_addr() -> String {
    std::env::var("DATABROKER_ADDR").unwrap_or_else(|_| "http://localhost:55556".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // TS-03-3: default address when DATABROKER_ADDR is not set
    #[test]
    #[serial]
    fn test_databroker_addr_default() {
        // Remove env var to test default fallback.
        std::env::remove_var("DATABROKER_ADDR");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://localhost:55556");
    }

    // TS-03-3: custom address from env
    #[test]
    #[serial]
    fn test_databroker_addr_env() {
        std::env::set_var("DATABROKER_ADDR", "http://10.0.0.5:55556");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://10.0.0.5:55556");
        // Clean up
        std::env::remove_var("DATABROKER_ADDR");
    }
}
