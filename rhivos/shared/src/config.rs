//! Configuration types for RHIVOS services

use serde::{Deserialize, Serialize};

/// Configuration for a RHIVOS service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    /// Unix Domain Socket path for local IPC
    pub grpc_socket_path: Option<String>,
    /// TCP address for network communication
    pub grpc_address: Option<String>,
    /// Whether TLS is enabled
    pub tls_enabled: bool,
    /// Path to TLS certificate
    pub tls_cert_path: Option<String>,
    /// Path to TLS private key
    pub tls_key_path: Option<String>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            grpc_socket_path: None,
            grpc_address: None,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

/// Configuration for connecting to the DATA_BROKER (Eclipse Kuksa)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBrokerConfig {
    /// Address of the data broker (socket path or TCP address)
    pub address: String,
    /// Whether to use TLS for the connection
    pub use_tls: bool,
}

impl Default for DataBrokerConfig {
    fn default() -> Self {
        Self {
            address: "/run/kuksa/databroker.sock".to_string(),
            use_tls: false,
        }
    }
}

/// Configuration for MQTT connections to CLOUD_GATEWAY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    /// MQTT broker URL
    pub broker_url: String,
    /// Client identifier
    pub client_id: String,
    /// Whether to use TLS
    pub use_tls: bool,
    /// Path to CA certificate for TLS verification
    pub ca_cert_path: Option<String>,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker_url: "mqtt://localhost:1883".to_string(),
            client_id: "rhivos-client".to_string(),
            use_tls: false,
            ca_cert_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_config_default() {
        let config = ServiceConfig::default();
        assert!(config.name.is_empty());
        assert!(!config.tls_enabled);
    }

    #[test]
    fn test_databroker_config_default() {
        let config = DataBrokerConfig::default();
        assert_eq!(config.address, "/run/kuksa/databroker.sock");
        assert!(!config.use_tls);
    }

    #[test]
    fn test_mqtt_config_default() {
        let config = MqttConfig::default();
        assert_eq!(config.broker_url, "mqtt://localhost:1883");
        assert!(!config.use_tls);
    }
}
