pub mod location;
pub mod speed;
pub mod door;

#[cfg(test)]
mod tests {
    #[test]
    fn test_modules_exist() {
        // Validates that all sensor modules compile
        assert!(true, "mock-sensors modules compile successfully");
    }
}
