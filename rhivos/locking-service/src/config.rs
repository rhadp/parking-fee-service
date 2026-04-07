/// Read the DATA_BROKER gRPC address from environment.
/// Falls back to `http://localhost:55556` if `DATABROKER_ADDR` is not set.
pub fn get_databroker_addr() -> String {
    todo!("get_databroker_addr not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-03-3: Verify default address when env var is not set.
    #[test]
    fn test_databroker_addr_default() {
        // Remove env var if set to test default behavior
        std::env::remove_var("DATABROKER_ADDR");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://localhost:55556");
    }

    /// TS-03-3: Verify custom address from env var.
    #[test]
    fn test_databroker_addr_env() {
        std::env::set_var("DATABROKER_ADDR", "http://10.0.0.5:55556");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://10.0.0.5:55556");
        // Clean up
        std::env::remove_var("DATABROKER_ADDR");
    }
}
