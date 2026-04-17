use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {e}"),
            ConfigError::Json(e) => write!(f, "JSON parse error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn default_config() -> Config {
    Config {
        grpc_port: default_grpc_port(),
        registry_url: String::new(),
        inactivity_timeout_secs: default_inactivity_timeout_secs(),
        container_storage_path: default_container_storage_path(),
    }
}

/// Load configuration from a JSON file at `path`.
/// If the file does not exist, returns `default_config()`.
/// If the file contains invalid JSON, returns an error.
pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let cfg: Config = serde_json::from_str(&contents).map_err(ConfigError::Json)?;
            Ok(cfg)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("Config file not found at {path}, using defaults");
            Ok(default_config())
        }
        Err(e) => Err(ConfigError::Io(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    // TS-07-14: Config loading from JSON file
    #[test]
    fn test_load_config_from_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(
            f,
            r#"{{"grpc_port":50099,"registry_url":"example.com","inactivity_timeout_secs":3600,"container_storage_path":"/tmp/adapters/"}}"#
        )
        .unwrap();
        let cfg = load_config(f.path().to_str().unwrap()).unwrap();
        assert_eq!(cfg.grpc_port, 50099);
        assert_eq!(cfg.registry_url, "example.com");
        assert_eq!(cfg.inactivity_timeout_secs, 3600);
        assert_eq!(cfg.container_storage_path, "/tmp/adapters/");
    }

    // TS-07-E13: Missing config file returns defaults
    #[test]
    fn test_config_file_missing_defaults() {
        let cfg = load_config("/nonexistent/path/config.json").unwrap();
        assert_eq!(cfg.grpc_port, 50052);
        assert_eq!(cfg.inactivity_timeout_secs, 86400);
        assert_eq!(cfg.container_storage_path, "/var/lib/containers/adapters/");
    }

    // TS-07-E14: Invalid JSON returns error
    #[test]
    fn test_config_invalid_json() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "{{invalid json").unwrap();
        let result = load_config(f.path().to_str().unwrap());
        assert!(result.is_err());
    }
}
