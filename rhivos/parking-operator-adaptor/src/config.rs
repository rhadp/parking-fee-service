//! Configuration for parking-operator-adaptor.
//!
//! This module defines service configuration loaded from environment variables.

use std::env;

/// Service configuration.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// TCP address for gRPC server
    pub listen_addr: String,
    /// TLS certificate path
    pub tls_cert_path: String,
    /// TLS key path
    pub tls_key_path: String,
    /// DATA_BROKER UDS socket path
    pub data_broker_socket: String,
    /// PARKING_OPERATOR base URL
    pub operator_base_url: String,
    /// PARKING_FEE_SERVICE base URL (for automatic zone lookup)
    pub parking_fee_service_url: String,
    /// Vehicle identifier
    pub vehicle_id: String,
    /// Configurable hourly rate (demo)
    pub hourly_rate: f64,
    /// Max retries for API calls
    pub api_max_retries: u32,
    /// Base delay for exponential backoff (ms)
    pub api_base_delay_ms: u64,
    /// Maximum delay for exponential backoff (ms)
    pub api_max_delay_ms: u64,
    /// API request timeout (ms)
    pub api_timeout_ms: u64,
    /// DATA_BROKER reconnect attempts
    pub reconnect_max_attempts: u32,
    /// DATA_BROKER reconnect base delay (ms)
    pub reconnect_base_delay_ms: u64,
    /// Maximum reconnect delay (ms)
    pub reconnect_max_delay_ms: u64,
    /// Status poll interval (seconds)
    pub poll_interval_seconds: u64,
    /// Session storage path
    pub storage_path: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:50053".to_string(),
            tls_cert_path: "/etc/rhivos/certs/parking-adaptor.crt".to_string(),
            tls_key_path: "/etc/rhivos/certs/parking-adaptor.key".to_string(),
            data_broker_socket: "/run/kuksa/databroker.sock".to_string(),
            operator_base_url: "http://localhost:8080/api/v1".to_string(),
            parking_fee_service_url: "http://localhost:8081/api/v1".to_string(),
            vehicle_id: "demo-vehicle-001".to_string(),
            hourly_rate: 2.50,
            api_max_retries: 3,
            api_base_delay_ms: 1000,
            api_max_delay_ms: 30000,
            api_timeout_ms: 10000,
            reconnect_max_attempts: 5,
            reconnect_base_delay_ms: 1000,
            reconnect_max_delay_ms: 30000,
            poll_interval_seconds: 60,
            storage_path: "/var/lib/parking-adaptor/session.json".to_string(),
        }
    }
}

impl ServiceConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = env::var("LISTEN_ADDR") {
            config.listen_addr = val;
        }
        if let Ok(val) = env::var("TLS_CERT_PATH") {
            config.tls_cert_path = val;
        }
        if let Ok(val) = env::var("TLS_KEY_PATH") {
            config.tls_key_path = val;
        }
        if let Ok(val) = env::var("DATA_BROKER_SOCKET") {
            config.data_broker_socket = val;
        }
        if let Ok(val) = env::var("OPERATOR_BASE_URL") {
            config.operator_base_url = val;
        }
        if let Ok(val) = env::var("PARKING_FEE_SERVICE_URL") {
            config.parking_fee_service_url = val;
        }
        if let Ok(val) = env::var("VEHICLE_ID") {
            config.vehicle_id = val;
        }
        if let Ok(val) = env::var("HOURLY_RATE") {
            if let Ok(rate) = val.parse() {
                config.hourly_rate = rate;
            }
        }
        if let Ok(val) = env::var("API_MAX_RETRIES") {
            if let Ok(retries) = val.parse() {
                config.api_max_retries = retries;
            }
        }
        if let Ok(val) = env::var("API_BASE_DELAY_MS") {
            if let Ok(delay) = val.parse() {
                config.api_base_delay_ms = delay;
            }
        }
        if let Ok(val) = env::var("API_MAX_DELAY_MS") {
            if let Ok(delay) = val.parse() {
                config.api_max_delay_ms = delay;
            }
        }
        if let Ok(val) = env::var("API_TIMEOUT_MS") {
            if let Ok(timeout) = val.parse() {
                config.api_timeout_ms = timeout;
            }
        }
        if let Ok(val) = env::var("RECONNECT_MAX_ATTEMPTS") {
            if let Ok(attempts) = val.parse() {
                config.reconnect_max_attempts = attempts;
            }
        }
        if let Ok(val) = env::var("RECONNECT_BASE_DELAY_MS") {
            if let Ok(delay) = val.parse() {
                config.reconnect_base_delay_ms = delay;
            }
        }
        if let Ok(val) = env::var("RECONNECT_MAX_DELAY_MS") {
            if let Ok(delay) = val.parse() {
                config.reconnect_max_delay_ms = delay;
            }
        }
        if let Ok(val) = env::var("POLL_INTERVAL_SECONDS") {
            if let Ok(interval) = val.parse() {
                config.poll_interval_seconds = interval;
            }
        }
        if let Ok(val) = env::var("STORAGE_PATH") {
            config.storage_path = val;
        }

        config
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.vehicle_id.is_empty() {
            return Err("VEHICLE_ID is required".to_string());
        }
        if self.operator_base_url.is_empty() {
            return Err("OPERATOR_BASE_URL is required".to_string());
        }
        if self.api_max_retries == 0 {
            return Err("API_MAX_RETRIES must be at least 1".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServiceConfig::default();
        assert_eq!(config.listen_addr, "0.0.0.0:50053");
        assert_eq!(config.api_max_retries, 3);
        assert_eq!(config.api_max_delay_ms, 30000);
        assert_eq!(config.reconnect_max_attempts, 5);
    }

    #[test]
    fn test_config_validation() {
        let config = ServiceConfig::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = config.clone();
        invalid_config.vehicle_id = String::new();
        assert!(invalid_config.validate().is_err());
    }
}
