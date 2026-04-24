use serde::Deserialize;
use std::fmt;

/// Service configuration loaded from a JSON file.
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    #[serde(default)]
    pub registry_url: String,
    #[serde(default = "default_inactivity_timeout_secs")]
    pub inactivity_timeout_secs: u64,
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

/// Error type for configuration loading.
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "config I/O error: {e}"),
            ConfigError::Parse(e) => write!(f, "config parse error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Returns a Config populated with all built-in defaults.
pub fn default_config() -> Config {
    Config {
        grpc_port: default_grpc_port(),
        registry_url: String::new(),
        inactivity_timeout_secs: default_inactivity_timeout_secs(),
        container_storage_path: default_container_storage_path(),
    }
}

/// Loads configuration from a JSON file at the given path.
///
/// If the file does not exist, returns `default_config()` and logs a warning.
/// If the file contains invalid JSON, returns a `ConfigError::Parse` error.
pub fn load_config(_path: &str) -> Result<Config, ConfigError> {
    todo!("load_config not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-07-14: Config loading from JSON file
    #[test]
    fn test_load_config_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{
                "grpc_port": 50099,
                "registry_url": "example.com",
                "inactivity_timeout_secs": 3600,
                "container_storage_path": "/tmp/adapters/"
            }"#,
        )
        .unwrap();
        let cfg = load_config(path.to_str().unwrap()).unwrap();
        assert_eq!(cfg.grpc_port, 50099);
        assert_eq!(cfg.registry_url, "example.com");
        assert_eq!(cfg.inactivity_timeout_secs, 3600);
        assert_eq!(cfg.container_storage_path, "/tmp/adapters/");
    }

    // TS-07-E13: Config file missing uses defaults
    #[test]
    fn test_config_file_missing_defaults() {
        let cfg = load_config("/nonexistent/path/config.json").unwrap();
        assert_eq!(cfg.grpc_port, 50052);
        assert_eq!(cfg.inactivity_timeout_secs, 86400);
        assert_eq!(cfg.container_storage_path, "/var/lib/containers/adapters/");
    }

    // TS-07-E14: Invalid JSON config returns error
    #[test]
    fn test_config_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "{invalid json").unwrap();
        let result = load_config(path.to_str().unwrap());
        assert!(result.is_err());
    }
}
