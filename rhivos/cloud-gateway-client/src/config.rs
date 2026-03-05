//! Configuration for the CLOUD_GATEWAY_CLIENT service.
//!
//! Parsed from environment variables at startup.

/// Service configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub vin: String,
    pub nats_url: String,
    pub nats_tls_enabled: bool,
    pub databroker_uds_path: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required: `VIN`
    /// Optional (with defaults):
    /// - `NATS_URL` -> `nats://localhost:4222`
    /// - `NATS_TLS_ENABLED` -> `false`
    /// - `DATABROKER_UDS_PATH` -> `/tmp/kuksa/databroker.sock`
    pub fn from_env() -> Result<Self, String> {
        let vin = std::env::var("VIN")
            .map_err(|_| "VIN environment variable is required but not set".to_string())?;

        let nats_url = std::env::var("NATS_URL")
            .unwrap_or_else(|_| "nats://localhost:4222".to_string());

        let nats_tls_enabled = std::env::var("NATS_TLS_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .map_err(|e| format!("NATS_TLS_ENABLED must be true or false: {e}"))?;

        let databroker_uds_path = std::env::var("DATABROKER_UDS_PATH")
            .unwrap_or_else(|_| "/tmp/kuksa/databroker.sock".to_string());

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
    use std::env;

    /// Helper to run config tests with isolated env vars.
    /// Restores original values after the closure runs.
    fn with_env_vars<F>(vars: &[(&str, Option<&str>)], f: F)
    where
        F: FnOnce(),
    {
        // Save originals
        let originals: Vec<_> = vars
            .iter()
            .map(|(key, _)| (*key, env::var(key).ok()))
            .collect();

        // Set or remove vars
        for (key, val) in vars {
            match val {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }

        f();

        // Restore originals
        for (key, original) in originals {
            match original {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }
    }

    /// TS-04-2: VIN is parsed from environment
    #[test]
    fn test_vin_parsed_from_env() {
        with_env_vars(
            &[
                ("VIN", Some("TEST_VIN_001")),
                ("NATS_URL", None),
                ("NATS_TLS_ENABLED", None),
                ("DATABROKER_UDS_PATH", None),
            ],
            || {
                let config = Config::from_env().expect("should parse config with VIN set");
                assert_eq!(config.vin, "TEST_VIN_001");
            },
        );
    }

    /// TS-04-2: Missing VIN produces an error
    #[test]
    fn test_missing_vin_produces_error() {
        with_env_vars(
            &[
                ("VIN", None),
                ("NATS_URL", None),
                ("NATS_TLS_ENABLED", None),
                ("DATABROKER_UDS_PATH", None),
            ],
            || {
                let result = Config::from_env();
                assert!(result.is_err(), "should error when VIN is not set");
                let err = result.unwrap_err();
                assert!(
                    err.to_lowercase().contains("vin"),
                    "error message should mention VIN, got: {err}"
                );
            },
        );
    }

    /// TS-04-3: NATS_URL defaults to nats://localhost:4222
    #[test]
    fn test_nats_url_default() {
        with_env_vars(
            &[
                ("VIN", Some("TEST_VIN_001")),
                ("NATS_URL", None),
                ("NATS_TLS_ENABLED", None),
                ("DATABROKER_UDS_PATH", None),
            ],
            || {
                let config = Config::from_env().expect("should parse config");
                assert_eq!(config.nats_url, "nats://localhost:4222");
            },
        );
    }

    /// TS-04-3: NATS_TLS_ENABLED defaults to false
    #[test]
    fn test_nats_tls_enabled_default() {
        with_env_vars(
            &[
                ("VIN", Some("TEST_VIN_001")),
                ("NATS_URL", None),
                ("NATS_TLS_ENABLED", None),
                ("DATABROKER_UDS_PATH", None),
            ],
            || {
                let config = Config::from_env().expect("should parse config");
                assert!(!config.nats_tls_enabled, "TLS should default to false");
            },
        );
    }

    /// TS-04-3: DATABROKER_UDS_PATH defaults to /tmp/kuksa/databroker.sock
    #[test]
    fn test_databroker_uds_path_default() {
        with_env_vars(
            &[
                ("VIN", Some("TEST_VIN_001")),
                ("NATS_URL", None),
                ("NATS_TLS_ENABLED", None),
                ("DATABROKER_UDS_PATH", None),
            ],
            || {
                let config = Config::from_env().expect("should parse config");
                assert_eq!(config.databroker_uds_path, "/tmp/kuksa/databroker.sock");
            },
        );
    }

    /// TS-04-3: Custom values are respected
    #[test]
    fn test_custom_env_values() {
        with_env_vars(
            &[
                ("VIN", Some("MY_VIN")),
                ("NATS_URL", Some("nats://custom:4222")),
                ("NATS_TLS_ENABLED", Some("true")),
                ("DATABROKER_UDS_PATH", Some("/custom/path.sock")),
            ],
            || {
                let config = Config::from_env().expect("should parse config");
                assert_eq!(config.vin, "MY_VIN");
                assert_eq!(config.nats_url, "nats://custom:4222");
                assert!(config.nats_tls_enabled);
                assert_eq!(config.databroker_uds_path, "/custom/path.sock");
            },
        );
    }
}
