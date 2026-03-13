//! Shared stub code for mock sensor binaries.
//!
//! This module will contain shared Databroker client code
//! once the full implementation is added.

/// Print usage information for a mock sensor binary.
pub fn print_usage(sensor_name: &str) {
    println!("{sensor_name} v0.1.0 - Mock sensor for vehicle signals");
    println!();
    println!("Usage: {sensor_name} [options]");
    println!();
    println!("This is a skeleton implementation. See spec 09 for full functionality.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_compiles() {
        // Verify the shared helper exists and is callable
        print_usage("test-sensor");
    }
}
