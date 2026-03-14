use serde::Deserialize;

/// Configuration for the UPDATE_SERVICE.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// gRPC listen port (default 50052).
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    /// Registry URL prefix for adapter images.
    #[serde(default)]
    pub registry_url: String,
    /// Inactivity timeout in seconds (default 86400 = 24 h).
    #[serde(default = "default_inactivity_timeout_secs")]
    pub inactivity_timeout_secs: u64,
    /// Container storage path (default `/var/lib/containers/adapters/`).
    #[serde(default = "default_container_storage_path")]
    pub container_storage_path: String,
}

fn default_grpc_port() -> u16 {
    50052
}

fn default_inactivity_timeout_secs() -> u64 {
    86400
}

fn default_container_storage_path() -> String {
    "/var/lib/containers/adapters/".to_string()
}

/// Errors that can occur during configuration loading.
#[derive(Debug)]
pub enum ConfigError {
    /// The JSON content is not valid.
    InvalidJson(serde_json::Error),
    /// An I/O error other than "not found" occurred.
    Io(std::io::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidJson(e) => write!(f, "invalid config JSON: {}", e),
            ConfigError::Io(e) => write!(f, "config I/O error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Return the built-in default configuration.
pub fn default_config() -> Config {
    Config {
        grpc_port: default_grpc_port(),
        registry_url: String::new(),
        inactivity_timeout_secs: default_inactivity_timeout_secs(),
        container_storage_path: default_container_storage_path(),
    }
}

/// Load configuration from `path`.
///
/// - If the file does not exist, returns `default_config()` (requirement 07-REQ-7.E1).
/// - If the file contains invalid JSON, returns `Err(ConfigError::InvalidJson)`.
/// - Missing JSON fields are filled from defaults (requirement 07-REQ-7.3).
pub fn load_config(_path: &str) -> Result<Config, ConfigError> {
    todo!("implement load_config")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_tmp(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().expect("temp file");
        f.write_all(content.as_bytes()).expect("write config");
        f
    }

    // TS-07-19: Config Loading — load_config reads values from a file.
    #[test]
    fn test_load_config_from_file() {
        let f = write_tmp(r#"{"grpc_port": 50053}"#);
        let cfg = load_config(f.path().to_str().unwrap())
            .expect("should load config");
        assert_eq!(cfg.grpc_port, 50053);
    }

    // TS-07-20: Config Fields — all fields are populated.
    #[test]
    fn test_config_fields() {
        let content = r#"{
            "grpc_port": 9090,
            "registry_url": "us-docker.pkg.dev/sdv-demo/adapters",
            "inactivity_timeout_secs": 3600,
            "container_storage_path": "/tmp/containers/"
        }"#;
        let f = write_tmp(content);
        let cfg = load_config(f.path().to_str().unwrap())
            .expect("should load config");
        assert!(cfg.grpc_port > 0, "grpc_port must be > 0");
        assert!(!cfg.registry_url.is_empty(), "registry_url must not be empty");
        assert!(cfg.inactivity_timeout_secs > 0, "timeout must be > 0");
        assert!(!cfg.container_storage_path.is_empty(), "storage path must not be empty");
        assert_eq!(cfg.grpc_port, 9090);
        assert_eq!(cfg.registry_url, "us-docker.pkg.dev/sdv-demo/adapters");
        assert_eq!(cfg.inactivity_timeout_secs, 3600);
        assert_eq!(cfg.container_storage_path, "/tmp/containers/");
    }

    // TS-07-21: Config Defaults — missing fields use built-in defaults.
    #[test]
    fn test_config_defaults() {
        let f = write_tmp("{}");
        let cfg = load_config(f.path().to_str().unwrap())
            .expect("empty config should succeed");
        assert_eq!(cfg.grpc_port, 50052, "default port must be 50052");
        assert_eq!(
            cfg.inactivity_timeout_secs, 86400,
            "default timeout must be 86400"
        );
        assert_eq!(
            cfg.container_storage_path, "/var/lib/containers/adapters/",
            "default storage path mismatch"
        );
    }

    // TS-07-17: Configurable Inactivity Timeout — loaded from config.
    #[test]
    fn test_configurable_inactivity() {
        let f = write_tmp(r#"{"inactivity_timeout_secs": 3600}"#);
        let cfg = load_config(f.path().to_str().unwrap())
            .expect("should load config");
        assert_eq!(cfg.inactivity_timeout_secs, 3600);
    }

    // TS-07-E9: Config File Missing — should return defaults, not error.
    #[test]
    fn test_config_file_missing() {
        let cfg = load_config("/nonexistent/path/to/config.json");
        // Must not return an error — falls back to defaults
        let cfg = cfg.expect("missing config file should return defaults");
        assert_eq!(cfg.grpc_port, 50052);
        assert_eq!(cfg.inactivity_timeout_secs, 86400);
        assert_eq!(
            cfg.container_storage_path,
            "/var/lib/containers/adapters/"
        );
    }

    // TS-07-E10: Config Invalid JSON — must return an error.
    #[test]
    fn test_config_invalid_json() {
        let f = write_tmp("{invalid json}");
        let result = load_config(f.path().to_str().unwrap());
        assert!(result.is_err(), "invalid JSON must return an error");
    }
}

// TS-07-P7: Config Defaults — property test.
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        // TS-07-P7: Any non-existent path returns valid defaults.
        #[test]
        #[ignore]
        fn proptest_config_defaults(
            path in "/nonexistent/[a-z0-9]{1,16}/config\\.json",
        ) {
            let cfg = load_config(&path).expect("missing file must return defaults");
            prop_assert_eq!(cfg.grpc_port, 50052);
            prop_assert_eq!(cfg.inactivity_timeout_secs, 86400);
            prop_assert_eq!(
                cfg.container_storage_path,
                "/var/lib/containers/adapters/"
            );
        }
    }
}
