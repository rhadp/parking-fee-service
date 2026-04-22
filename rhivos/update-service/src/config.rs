use serde::Deserialize;

/// Service configuration loaded from a JSON file.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub grpc_port: u16,
    pub registry_url: String,
    pub inactivity_timeout_secs: u64,
    pub container_storage_path: String,
}

/// Errors that can occur when loading configuration.
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "config I/O error: {e}"),
            Self::Parse(e) => write!(f, "config parse error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Load configuration from a JSON file at the given path.
///
/// If the file does not exist, returns default configuration.
/// If the file contains invalid JSON, returns an error.
pub fn load_config(_path: &str) -> Result<Config, ConfigError> {
    todo!()
}

/// Returns the built-in default configuration.
pub fn default_config() -> Config {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // TS-07-14: Config loading from file
    #[test]
    fn test_load_config_from_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_load_config_07_14.json");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            r#"{{"grpc_port":50099,"registry_url":"example.com","inactivity_timeout_secs":3600,"container_storage_path":"/tmp/adapters/"}}"#
        )
        .unwrap();

        let cfg = load_config(path.to_str().unwrap()).unwrap();
        assert_eq!(cfg.grpc_port, 50099);
        assert_eq!(cfg.registry_url, "example.com");
        assert_eq!(cfg.inactivity_timeout_secs, 3600);
        assert_eq!(cfg.container_storage_path, "/tmp/adapters/");

        let _ = std::fs::remove_file(&path);
    }

    // TS-07-E13: Config file missing uses defaults
    #[test]
    fn test_config_file_missing_defaults() {
        let cfg = load_config("/nonexistent/path/config_07_e13.json").unwrap();
        assert_eq!(cfg.grpc_port, 50052);
        assert_eq!(cfg.inactivity_timeout_secs, 86400);
        assert_eq!(cfg.container_storage_path, "/var/lib/containers/adapters/");
    }

    // TS-07-E14: Invalid JSON config returns error
    #[test]
    fn test_config_invalid_json() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_invalid_config_07_e14.json");
        std::fs::write(&path, "{invalid json").unwrap();

        let result = load_config(path.to_str().unwrap());
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
    }
}
