//! Unit tests for the config module.
//!
//! Test Spec: TS-04-1, TS-04-2, TS-04-E1
//! Requirements: 04-REQ-1.1, 04-REQ-1.2, 04-REQ-1.3, 04-REQ-1.4, 04-REQ-1.E1

use cloud_gateway_client::config::Config;
use cloud_gateway_client::errors::ConfigError;
use serial_test::serial;

/// TS-04-1: Config reads VIN from environment and applies defaults.
///
/// Requirement: 04-REQ-1.1
/// WHEN the VIN environment variable is set, the system SHALL use its value.
/// WHEN optional variables are not set, the system SHALL use defaults.
#[test]
#[serial]
fn test_config_reads_vin_from_env() {
    std::env::set_var("VIN", "TEST-VIN-001");
    std::env::remove_var("NATS_URL");
    std::env::remove_var("DATABROKER_ADDR");
    std::env::remove_var("BEARER_TOKEN");

    let config = Config::from_env().expect("Config should succeed with VIN set");

    assert_eq!(config.vin, "TEST-VIN-001");
    assert_eq!(config.nats_url, "nats://localhost:4222");
    assert_eq!(config.databroker_addr, "http://localhost:55556");
    assert_eq!(config.bearer_token, "demo-token");
}

/// TS-04-2: Config reads all custom environment variables.
///
/// Requirements: 04-REQ-1.2, 04-REQ-1.3, 04-REQ-1.4
/// WHEN custom environment variables are set, the system SHALL use their values.
#[test]
#[serial]
fn test_config_reads_all_custom_env() {
    std::env::set_var("VIN", "MY-VIN");
    std::env::set_var("NATS_URL", "nats://custom:9222");
    std::env::set_var("DATABROKER_ADDR", "http://custom:55557");
    std::env::set_var("BEARER_TOKEN", "secret-token");

    let config = Config::from_env().expect("Config should succeed with all vars set");

    assert_eq!(config.vin, "MY-VIN");
    assert_eq!(config.nats_url, "nats://custom:9222");
    assert_eq!(config.databroker_addr, "http://custom:55557");
    assert_eq!(config.bearer_token, "secret-token");
}

/// TS-04-E1: Config fails when VIN is missing.
///
/// Requirement: 04-REQ-1.E1
/// WHEN the VIN environment variable is not set, the system SHALL return
/// an error indicating the VIN is missing.
#[test]
#[serial]
fn test_config_missing_vin() {
    std::env::remove_var("VIN");
    std::env::remove_var("NATS_URL");
    std::env::remove_var("DATABROKER_ADDR");
    std::env::remove_var("BEARER_TOKEN");

    let result = Config::from_env();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ConfigError::MissingVin);
}
