/// Configuration for the CLOUD_GATEWAY_CLIENT.
#[derive(Debug)]
pub struct Config {
    /// Vehicle Identification Number; used in NATS subject hierarchy.
    pub vin: String,
    /// NATS server connection URL.
    pub nats_url: String,
    /// Enable TLS for NATS connection.
    pub nats_tls_enabled: bool,
    /// Unix Domain Socket path for DATA_BROKER gRPC connection.
    pub databroker_uds_path: String,
}

/// Error returned when required configuration is missing.
#[derive(Debug)]
pub struct ConfigError {
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Config {
    /// Default NATS URL.
    pub const DEFAULT_NATS_URL: &'static str = "nats://localhost:4222";
    /// Default UDS path for DATA_BROKER.
    pub const DEFAULT_UDS_PATH: &'static str = "/tmp/kuksa/databroker.sock";

    /// Parse configuration from environment variables.
    ///
    /// Returns an error if `VIN` is not set.
    pub fn from_env() -> Result<Self, ConfigError> {
        let vin = std::env::var("VIN").map_err(|_| ConfigError {
            message: "VIN environment variable is required but not set".to_string(),
        })?;

        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| Self::DEFAULT_NATS_URL.to_string());

        let nats_tls_enabled = std::env::var("NATS_TLS_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let databroker_uds_path = std::env::var("DATABROKER_UDS_PATH")
            .unwrap_or_else(|_| Self::DEFAULT_UDS_PATH.to_string());

        Ok(Config {
            vin,
            nats_url,
            nats_tls_enabled,
            databroker_uds_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-04-2: VIN environment variable is required
    #[test]
    fn test_missing_vin_returns_error() {
        std::env::remove_var("VIN");
        let result = Config::from_env();
        assert!(
            result.is_err(),
            "Config should return an error when VIN is not set"
        );
        let err = result.unwrap_err();
        assert!(
            err.message.to_lowercase().contains("vin"),
            "Error message should mention VIN, got: {}",
            err.message
        );
    }

    // TS-04-2: VIN is parsed from environment
    #[test]
    fn test_vin_parsed_from_env() {
        std::env::set_var("VIN", "WVWZZZ3CZWE123456");
        // Clear optional vars to use defaults
        std::env::remove_var("NATS_URL");
        std::env::remove_var("NATS_TLS_ENABLED");
        std::env::remove_var("DATABROKER_UDS_PATH");
        let config = Config::from_env().expect("Config should succeed when VIN is set");
        std::env::remove_var("VIN");
        assert_eq!(config.vin, "WVWZZZ3CZWE123456");
    }

    // TS-04-3: NATS_URL defaults to nats://localhost:4222
    #[test]
    fn test_nats_url_default() {
        std::env::set_var("VIN", "TEST_VIN_DEFAULT");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("NATS_TLS_ENABLED");
        std::env::remove_var("DATABROKER_UDS_PATH");
        let config = Config::from_env().expect("Config should succeed");
        std::env::remove_var("VIN");
        assert_eq!(
            config.nats_url, "nats://localhost:4222",
            "NATS_URL should default to nats://localhost:4222"
        );
    }

    // TS-04-3: NATS_TLS_ENABLED defaults to false
    #[test]
    fn test_nats_tls_enabled_default() {
        std::env::set_var("VIN", "TEST_VIN_TLS");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("NATS_TLS_ENABLED");
        std::env::remove_var("DATABROKER_UDS_PATH");
        let config = Config::from_env().expect("Config should succeed");
        std::env::remove_var("VIN");
        assert!(
            !config.nats_tls_enabled,
            "NATS_TLS_ENABLED should default to false"
        );
    }

    // TS-04-3: DATABROKER_UDS_PATH defaults to /tmp/kuksa/databroker.sock
    #[test]
    fn test_databroker_uds_path_default() {
        std::env::set_var("VIN", "TEST_VIN_UDS");
        std::env::remove_var("NATS_URL");
        std::env::remove_var("NATS_TLS_ENABLED");
        std::env::remove_var("DATABROKER_UDS_PATH");
        let config = Config::from_env().expect("Config should succeed");
        std::env::remove_var("VIN");
        assert_eq!(
            config.databroker_uds_path, "/tmp/kuksa/databroker.sock",
            "DATABROKER_UDS_PATH should default to /tmp/kuksa/databroker.sock"
        );
    }
}
