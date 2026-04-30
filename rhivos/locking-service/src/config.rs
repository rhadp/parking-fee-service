/// Default DATA_BROKER gRPC address.
const DEFAULT_DATABROKER_ADDR: &str = "http://localhost:55556";

/// Returns the DATA_BROKER gRPC address.
///
/// Reads from `DATABROKER_ADDR` environment variable. Falls back to
/// `http://localhost:55556` if not set.
pub fn get_databroker_addr() -> String {
    std::env::var("DATABROKER_ADDR").unwrap_or_else(|_| DEFAULT_DATABROKER_ADDR.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-3 Case 1: Verify default address when DATABROKER_ADDR is not set.
    #[test]
    fn test_databroker_addr_default() {
        // Remove the env var if set to test the default.
        std::env::remove_var("DATABROKER_ADDR");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://localhost:55556");
    }

    // TS-03-3 Case 2: Verify custom address from environment variable.
    #[test]
    fn test_databroker_addr_env() {
        std::env::set_var("DATABROKER_ADDR", "http://10.0.0.5:55556");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://10.0.0.5:55556");
        // Clean up to avoid affecting other tests.
        std::env::remove_var("DATABROKER_ADDR");
    }
}
