//! Service configuration.
//!
//! Reads `DATABROKER_ADDR` from the environment with a fallback default.

/// Default DATA_BROKER gRPC address when `DATABROKER_ADDR` is not set.
pub const DEFAULT_DATABROKER_ADDR: &str = "http://localhost:55556";

/// Return the DATA_BROKER gRPC address.
///
/// Reads from the `DATABROKER_ADDR` environment variable.
/// Falls back to `http://localhost:55556` if the variable is absent (03-REQ-7.2).
pub fn get_databroker_addr() -> String {
    std::env::var("DATABROKER_ADDR").unwrap_or_else(|_| DEFAULT_DATABROKER_ADDR.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialise all env-var tests to prevent races when cargo test runs
    // tests in parallel on multiple threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // TS-03-3 (case 1): Default address when DATABROKER_ADDR is not set
    #[test]
    fn test_databroker_addr_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("DATABROKER_ADDR");
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://localhost:55556");
    }

    // TS-03-3 (case 2): Custom address from DATABROKER_ADDR environment variable
    #[test]
    fn test_databroker_addr_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("DATABROKER_ADDR", "http://10.0.0.5:55556");
        let addr = get_databroker_addr();
        // Clean up before asserting to avoid polluting other tests.
        std::env::remove_var("DATABROKER_ADDR");
        assert_eq!(addr, "http://10.0.0.5:55556");
    }
}
