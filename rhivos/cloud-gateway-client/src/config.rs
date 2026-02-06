//! Configuration management for cloud-gateway-client.
//!
//! Configuration is loaded from environment variables and validated on startup.

use std::env;
use std::path::PathBuf;

use crate::error::CloudGatewayError;

/// Service configuration for CLOUD_GATEWAY_CLIENT.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Vehicle Identification Number
    pub vin: String,

    /// MQTT broker configuration
    pub mqtt: MqttConfig,

    /// LOCKING_SERVICE UDS socket path
    pub locking_service_socket: String,

    /// DATA_BROKER UDS socket path
    pub data_broker_socket: String,

    /// Valid auth tokens (demo-grade)
    pub valid_tokens: Vec<String>,

    /// Command processing timeout in milliseconds
    pub command_timeout_ms: u64,

    /// Telemetry publish interval in milliseconds
    pub telemetry_interval_ms: u64,

    /// Graceful shutdown timeout in milliseconds
    pub shutdown_timeout_ms: u64,
}

/// MQTT broker configuration.
#[derive(Debug, Clone)]
pub struct MqttConfig {
    /// MQTT broker URL (mqtts://host:port)
    pub broker_url: String,

    /// Client ID for MQTT connection
    pub client_id: String,

    /// Path to TLS CA certificate
    pub ca_cert_path: PathBuf,

    /// Path to client certificate
    pub client_cert_path: PathBuf,

    /// Path to client private key
    pub client_key_path: PathBuf,

    /// Keepalive interval in seconds
    pub keepalive_secs: u64,

    /// Initial reconnect delay in milliseconds
    pub reconnect_initial_delay_ms: u64,

    /// Maximum reconnect delay in milliseconds
    pub reconnect_max_delay_ms: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            vin: "DEMO_VIN_001".to_string(),
            mqtt: MqttConfig::default(),
            locking_service_socket: "/run/rhivos/locking.sock".to_string(),
            data_broker_socket: "/run/kuksa/databroker.sock".to_string(),
            valid_tokens: vec!["demo-token".to_string()],
            command_timeout_ms: 5000,
            telemetry_interval_ms: 1000,
            shutdown_timeout_ms: 10000,
        }
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker_url: "mqtts://localhost:8883".to_string(),
            client_id: "cloud-gateway-client".to_string(),
            ca_cert_path: PathBuf::from("/etc/rhivos/certs/ca.crt"),
            client_cert_path: PathBuf::from("/etc/rhivos/certs/client.crt"),
            client_key_path: PathBuf::from("/etc/rhivos/certs/client.key"),
            keepalive_secs: 30,
            reconnect_initial_delay_ms: 1000,
            reconnect_max_delay_ms: 60000,
        }
    }
}

impl ServiceConfig {
    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `CGC_VIN`: Vehicle Identification Number
    /// - `CGC_MQTT_BROKER_URL`: MQTT broker URL
    /// - `CGC_MQTT_CLIENT_ID`: MQTT client ID
    /// - `CGC_MQTT_CA_CERT_PATH`: Path to CA certificate
    /// - `CGC_MQTT_CLIENT_CERT_PATH`: Path to client certificate
    /// - `CGC_MQTT_CLIENT_KEY_PATH`: Path to client key
    /// - `CGC_MQTT_KEEPALIVE_SECS`: Keepalive interval
    /// - `CGC_MQTT_RECONNECT_INITIAL_MS`: Initial reconnect delay
    /// - `CGC_MQTT_RECONNECT_MAX_MS`: Maximum reconnect delay
    /// - `CGC_LOCKING_SERVICE_SOCKET`: LOCKING_SERVICE socket path
    /// - `CGC_DATA_BROKER_SOCKET`: DATA_BROKER socket path
    /// - `CGC_VALID_TOKENS`: Comma-separated list of valid tokens
    /// - `CGC_COMMAND_TIMEOUT_MS`: Command timeout
    /// - `CGC_TELEMETRY_INTERVAL_MS`: Telemetry interval
    /// - `CGC_SHUTDOWN_TIMEOUT_MS`: Shutdown timeout
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(vin) = env::var("CGC_VIN") {
            config.vin = vin;
        }

        if let Ok(url) = env::var("CGC_MQTT_BROKER_URL") {
            config.mqtt.broker_url = url;
        }

        if let Ok(id) = env::var("CGC_MQTT_CLIENT_ID") {
            config.mqtt.client_id = id;
        }

        if let Ok(path) = env::var("CGC_MQTT_CA_CERT_PATH") {
            config.mqtt.ca_cert_path = PathBuf::from(path);
        }

        if let Ok(path) = env::var("CGC_MQTT_CLIENT_CERT_PATH") {
            config.mqtt.client_cert_path = PathBuf::from(path);
        }

        if let Ok(path) = env::var("CGC_MQTT_CLIENT_KEY_PATH") {
            config.mqtt.client_key_path = PathBuf::from(path);
        }

        if let Ok(secs) = env::var("CGC_MQTT_KEEPALIVE_SECS") {
            if let Ok(v) = secs.parse() {
                config.mqtt.keepalive_secs = v;
            }
        }

        if let Ok(ms) = env::var("CGC_MQTT_RECONNECT_INITIAL_MS") {
            if let Ok(v) = ms.parse() {
                config.mqtt.reconnect_initial_delay_ms = v;
            }
        }

        if let Ok(ms) = env::var("CGC_MQTT_RECONNECT_MAX_MS") {
            if let Ok(v) = ms.parse() {
                config.mqtt.reconnect_max_delay_ms = v;
            }
        }

        if let Ok(path) = env::var("CGC_LOCKING_SERVICE_SOCKET") {
            config.locking_service_socket = path;
        }

        if let Ok(path) = env::var("CGC_DATA_BROKER_SOCKET") {
            config.data_broker_socket = path;
        }

        if let Ok(tokens) = env::var("CGC_VALID_TOKENS") {
            config.valid_tokens = tokens.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(ms) = env::var("CGC_COMMAND_TIMEOUT_MS") {
            if let Ok(v) = ms.parse() {
                config.command_timeout_ms = v;
            }
        }

        if let Ok(ms) = env::var("CGC_TELEMETRY_INTERVAL_MS") {
            if let Ok(v) = ms.parse() {
                config.telemetry_interval_ms = v;
            }
        }

        if let Ok(ms) = env::var("CGC_SHUTDOWN_TIMEOUT_MS") {
            if let Ok(v) = ms.parse() {
                config.shutdown_timeout_ms = v;
            }
        }

        config
    }

    /// Validate the configuration and return an error if invalid.
    pub fn validate(&self) -> Result<(), CloudGatewayError> {
        if self.vin.is_empty() {
            return Err(CloudGatewayError::ConfigError(
                "VIN cannot be empty".to_string(),
            ));
        }

        if !self.mqtt.broker_url.starts_with("mqtt://")
            && !self.mqtt.broker_url.starts_with("mqtts://")
        {
            return Err(CloudGatewayError::ConfigError(format!(
                "Invalid broker URL: {} (must start with mqtt:// or mqtts://)",
                self.mqtt.broker_url
            )));
        }

        if self.valid_tokens.is_empty() {
            return Err(CloudGatewayError::ConfigError(
                "At least one valid token is required".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_default_config() {
        let config = ServiceConfig::default();
        assert_eq!(config.vin, "DEMO_VIN_001");
        assert_eq!(config.mqtt.broker_url, "mqtts://localhost:8883");
        assert_eq!(config.command_timeout_ms, 5000);
        assert_eq!(config.shutdown_timeout_ms, 10000);
    }

    #[test]
    fn test_validate_empty_vin() {
        let mut config = ServiceConfig::default();
        config.vin = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_broker_url() {
        let mut config = ServiceConfig::default();
        config.mqtt.broker_url = "http://localhost:8883".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_empty_tokens() {
        let mut config = ServiceConfig::default();
        config.valid_tokens = vec![];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_success() {
        let config = ServiceConfig::default();
        assert!(config.validate().is_ok());
    }

    // Property 15: Configuration Validation
    // Validates: Requirements 8.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_empty_vin_fails_validation(
            broker_url in "(mqtt|mqtts)://[a-z]+:[0-9]+",
            token in "[a-zA-Z0-9-]{1,32}"
        ) {
            let mut config = ServiceConfig::default();
            config.vin = "".to_string();
            config.mqtt.broker_url = broker_url;
            config.valid_tokens = vec![token];

            // Empty VIN should fail validation
            prop_assert!(config.validate().is_err());
            let err = config.validate().unwrap_err().to_string();
            prop_assert!(err.contains("VIN"));
        }

        #[test]
        fn prop_invalid_broker_url_fails_validation(
            vin in "[A-Z0-9]{17}",
            protocol in "(http|https|ftp|tcp)",
            host in "[a-z]+:[0-9]+"
        ) {
            let mut config = ServiceConfig::default();
            config.vin = vin;
            config.mqtt.broker_url = format!("{}://{}", protocol, host);

            // Invalid broker URL should fail validation
            prop_assert!(config.validate().is_err());
            let err = config.validate().unwrap_err().to_string();
            prop_assert!(err.contains("broker URL") || err.contains("mqtt"));
        }

        #[test]
        fn prop_empty_tokens_fails_validation(
            vin in "[A-Z0-9]{17}",
            broker_url in "(mqtt|mqtts)://[a-z]+:[0-9]+"
        ) {
            let mut config = ServiceConfig::default();
            config.vin = vin;
            config.mqtt.broker_url = broker_url;
            config.valid_tokens = vec![];

            // Empty tokens should fail validation
            prop_assert!(config.validate().is_err());
            let err = config.validate().unwrap_err().to_string();
            prop_assert!(err.contains("token"));
        }

        #[test]
        fn prop_valid_config_passes_validation(
            vin in "[A-Z0-9]{17}",
            broker_url in "(mqtt|mqtts)://[a-z]+:[0-9]+",
            token in "[a-zA-Z0-9-]{1,32}"
        ) {
            let mut config = ServiceConfig::default();
            config.vin = vin;
            config.mqtt.broker_url = broker_url;
            config.valid_tokens = vec![token];

            // Valid config should pass validation
            prop_assert!(config.validate().is_ok());
        }
    }
}
