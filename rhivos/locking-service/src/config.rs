/// Read the DATA_BROKER gRPC address from environment.
///
/// Returns the value of `DATABROKER_ADDR` if set,
/// otherwise returns `http://localhost:55556`.
pub fn get_databroker_addr() -> String {
    std::env::var("DATABROKER_ADDR").unwrap_or_else(|_| "http://localhost:55556".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-03-3: Verify default and custom address from DATABROKER_ADDR env var.
    //
    // Both checks run in a single test to avoid a race condition: when run in
    // parallel, set_var/remove_var on a shared env var causes data races.
    #[test]
    fn test_databroker_addr() {
        // Test custom address first.
        // SAFETY: env var mutation in single-threaded test context.
        unsafe { std::env::set_var("DATABROKER_ADDR", "http://10.0.0.5:55556") };
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://10.0.0.5:55556");

        // Now test default by removing the env var.
        unsafe { std::env::remove_var("DATABROKER_ADDR") };
        let addr = get_databroker_addr();
        assert_eq!(addr, "http://localhost:55556");
    }
}
