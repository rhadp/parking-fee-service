//! Service configuration.
//!
//! Reads the DATA_BROKER gRPC address from the environment (03-REQ-7.1),
//! defaulting to `http://localhost:55556` (03-REQ-7.2).

#![allow(dead_code)]

/// Return the DATA_BROKER gRPC address.
///
/// Reads `DATABROKER_ADDR` from the environment. Falls back to
/// `http://localhost:55556` when the variable is unset.
pub fn get_databroker_addr() -> String {
    std::env::var("DATABROKER_ADDR").unwrap_or_else(|_| "http://localhost:55556".to_string())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-03-3 / 03-REQ-7.2: Default address when DATABROKER_ADDR is not set.
    #[test]
    fn test_databroker_addr_default() {
        // SAFETY: test isolation is the caller's responsibility. Each test run
        // should arrange env state before asserting.
        std::env::remove_var("DATABROKER_ADDR");
        let addr = get_databroker_addr();
        assert_eq!(
            addr, "http://localhost:55556",
            "default address must be http://localhost:55556"
        );
    }

    /// TS-03-3 / 03-REQ-7.1: Custom address read from DATABROKER_ADDR env var.
    #[test]
    fn test_databroker_addr_env() {
        std::env::set_var("DATABROKER_ADDR", "http://10.0.0.5:55556");
        let addr = get_databroker_addr();
        std::env::remove_var("DATABROKER_ADDR");
        assert_eq!(
            addr, "http://10.0.0.5:55556",
            "must return the value from DATABROKER_ADDR"
        );
    }
}
