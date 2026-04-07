use serde::Deserialize;
use std::fmt;

/// Service configuration loaded from a JSON file.
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    #[serde(default)]
    pub registry_url: String,
    #[serde(default = "default_inactivity_timeout")]
    pub inactivity_timeout_secs: u64,
    #[serde(default = "default_container_storage_path")]
    pub container_storage_path: String,
}

fn default_grpc_port() -> u16 {
    50052
}

fn default_inactivity_timeout() -> u64 {
    86400
}

fn default_container_storage_path() -> String {
    "/var/lib/containers/adapters/".to_string()
}

/// Errors that can occur when loading configuration.
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "config IO error: {e}"),
            ConfigError::Parse(e) => write!(f, "config parse error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Load configuration from the given JSON file path.
///
/// If the file does not exist, returns a default configuration.
/// If the file contains invalid JSON, returns an error.
pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let config: Config = serde_json::from_str(&contents).map_err(ConfigError::Parse)?;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("Config file not found at {path}, using defaults");
            Ok(default_config())
        }
        Err(e) => Err(ConfigError::Io(e)),
    }
}

/// Return the built-in default configuration.
pub fn default_config() -> Config {
    Config {
        grpc_port: default_grpc_port(),
        registry_url: String::new(),
        inactivity_timeout_secs: default_inactivity_timeout(),
        container_storage_path: default_container_storage_path(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // TS-07-14: Config Loading From File
    #[test]
    fn test_load_config_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        {
            let mut f = std::fs::File::create(&config_path).unwrap();
            f.write_all(
                br#"{"grpc_port":50099,"registry_url":"example.com","inactivity_timeout_secs":3600,"container_storage_path":"/tmp/adapters/"}"#,
            )
            .unwrap();
        }

        let cfg = load_config(config_path.to_str().unwrap()).expect("should load config");
        assert_eq!(cfg.grpc_port, 50099);
        assert_eq!(cfg.registry_url, "example.com");
        assert_eq!(cfg.inactivity_timeout_secs, 3600);
        assert_eq!(cfg.container_storage_path, "/tmp/adapters/");
    }

    // TS-07-E13: Config File Missing Uses Defaults
    #[test]
    fn test_config_file_missing_defaults() {
        let cfg = load_config("/nonexistent/path/config.json").expect("should return defaults");
        assert_eq!(cfg.grpc_port, 50052);
        assert_eq!(cfg.inactivity_timeout_secs, 86400);
        assert_eq!(cfg.container_storage_path, "/var/lib/containers/adapters/");
    }

    // TS-07-E14: Invalid JSON Config Exits With Error
    #[test]
    fn test_config_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("bad.json");
        {
            let mut f = std::fs::File::create(&config_path).unwrap();
            f.write_all(b"{invalid json").unwrap();
        }

        let result = load_config(config_path.to_str().unwrap());
        assert!(result.is_err(), "invalid JSON should produce an error");
    }
}
