//! Mock sensor library providing simulated vehicle signal inputs.
//!
//! # Example
//!
//! ```
//! // mock-sensors crate provides location, speed, and door modules
//! use mock_sensors::location;
//! use mock_sensors::speed;
//! use mock_sensors::door;
//! ```

pub mod location;
pub mod speed;
pub mod door;

#[cfg(test)]
mod tests {
    #[test]
    fn test_modules_exist() {
        // Validates that all sensor modules compile
        assert!(true, "mock-sensors library compiles with all modules");
    }
}
