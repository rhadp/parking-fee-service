//! Unit tests for the `config` module.
//!
//! Tests cover:
//! - TS-04-1: Config reads VIN from environment
//! - TS-04-2: Config reads all custom environment variables
//! - TS-04-E1: Config fails when VIN is missing
//!
//! Requirements: [04-REQ-1.1], [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4], [04-REQ-1.E1]

use cloud_gateway_client::config::Config;
use cloud_gateway_client::errors::ConfigError;
use serial_test::serial;

// ---------------------------------------------------------------------------
// TS-04-1: Config reads VIN from environment
// Validates: [04-REQ-1.1]
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn ts_04_1_config_reads_vin_with_defaults() {
    // GIVEN env VIN="TEST-VIN-001"
    // GIVEN env NATS_URL not set
    // GIVEN env DATABROKER_ADDR not set
    // GIVEN env BEARER_TOKEN not set
    std::env::set_var("VIN", "TEST-VIN-001");
    std::env::remove_var("NATS_URL");
    std::env::remove_var("DATABROKER_ADDR");
    std::env::remove_var("BEARER_TOKEN");

    // WHEN Config::from_env() is called
    let result = Config::from_env();

    // THEN result is Ok(config)
    let config = result.expect("Config::from_env() should succeed when VIN is set");

    // AND config.vin == "TEST-VIN-001"
    assert_eq!(config.vin, "TEST-VIN-001");
    // AND config.nats_url == "nats://localhost:4222"
    assert_eq!(config.nats_url, "nats://localhost:4222");
    // AND config.databroker_addr == "http://localhost:55556"
    assert_eq!(config.databroker_addr, "http://localhost:55556");
    // AND config.bearer_token == "demo-token"
    assert_eq!(config.bearer_token, "demo-token");
}

// ---------------------------------------------------------------------------
// TS-04-2: Config reads all custom environment variables
// Validates: [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4]
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn ts_04_2_config_reads_custom_env_vars() {
    // GIVEN env VIN="MY-VIN"
    // GIVEN env NATS_URL="nats://custom:9222"
    // GIVEN env DATABROKER_ADDR="http://custom:55557"
    // GIVEN env BEARER_TOKEN="secret-token"
    std::env::set_var("VIN", "MY-VIN");
    std::env::set_var("NATS_URL", "nats://custom:9222");
    std::env::set_var("DATABROKER_ADDR", "http://custom:55557");
    std::env::set_var("BEARER_TOKEN", "secret-token");

    // WHEN Config::from_env() is called
    let result = Config::from_env();

    // THEN result is Ok(config)
    let config = result.expect("Config::from_env() should succeed with all env vars set");

    // AND config.nats_url == "nats://custom:9222"
    assert_eq!(config.nats_url, "nats://custom:9222");
    // AND config.databroker_addr == "http://custom:55557"
    assert_eq!(config.databroker_addr, "http://custom:55557");
    // AND config.bearer_token == "secret-token"
    assert_eq!(config.bearer_token, "secret-token");

    // Clean up
    std::env::remove_var("NATS_URL");
    std::env::remove_var("DATABROKER_ADDR");
    std::env::remove_var("BEARER_TOKEN");
}

// ---------------------------------------------------------------------------
// TS-04-E1: Config fails when VIN is missing
// Validates: [04-REQ-1.E1]
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn ts_04_e1_config_fails_missing_vin() {
    // GIVEN env VIN is not set
    std::env::remove_var("VIN");
    std::env::remove_var("NATS_URL");
    std::env::remove_var("DATABROKER_ADDR");
    std::env::remove_var("BEARER_TOKEN");

    // WHEN Config::from_env() is called
    let result = Config::from_env();

    // THEN result is Err(ConfigError::MissingVin)
    assert_eq!(result.unwrap_err(), ConfigError::MissingVin);
}
