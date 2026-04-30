use serde::Deserialize;

/// Service configuration.
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

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "config I/O error: {e}"),
            ConfigError::Parse(e) => write!(f, "config parse error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Returns the built-in default configuration.
pub fn default_config() -> Config {
    Config {
        grpc_port: default_grpc_port(),
        registry_url: String::new(),
        inactivity_timeout_secs: default_inactivity_timeout_secs(),
        container_storage_path: default_container_storage_path(),
    }
}

/// Loads configuration from the given JSON file path.
///
/// If the file does not exist, returns the default configuration.
/// If the file contains invalid JSON, returns an error.
pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("config file not found at {path}, using defaults");
            return Ok(default_config());
        }
        Err(e) => return Err(ConfigError::Io(e)),
    };
    serde_json::from_str(&contents).map_err(ConfigError::Parse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // TS-07-14: Config Loading From File
    // Requirements: 07-REQ-7.1, 07-REQ-7.2
    #[test]
    fn test_load_config_from_file() {
        let dir = std::env::temp_dir().join("update-service-test-config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(
            br#"{"grpc_port":50099,"registry_url":"example.com","inactivity_timeout_secs":3600,"container_storage_path":"/tmp/adapters/"}"#,
        )
        .unwrap();
        drop(f);

        let cfg = load_config(path.to_str().unwrap()).expect("should load");
        assert_eq!(cfg.grpc_port, 50099);
        assert_eq!(cfg.registry_url, "example.com");
        assert_eq!(cfg.inactivity_timeout_secs, 3600);
        assert_eq!(cfg.container_storage_path, "/tmp/adapters/");

        std::fs::remove_dir_all(&dir).ok();
    }

    // TS-07-E13: Config File Missing Uses Defaults
    // Requirement: 07-REQ-7.E1
    #[test]
    fn test_config_file_missing_defaults() {
        let cfg = load_config("/nonexistent/path/config.json")
            .expect("should return defaults when file is missing");
        assert_eq!(cfg.grpc_port, 50052);
        assert_eq!(cfg.inactivity_timeout_secs, 86400);
        assert_eq!(cfg.container_storage_path, "/var/lib/containers/adapters/");
    }

    // TS-07-E14: Invalid JSON Config Exits With Error
    // Requirement: 07-REQ-7.E2
    #[test]
    fn test_config_invalid_json() {
        let dir = std::env::temp_dir().join("update-service-test-invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("invalid-config.json");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"{invalid json").unwrap();
        drop(f);

        let result = load_config(path.to_str().unwrap());
        assert!(result.is_err(), "invalid JSON should return error");

        std::fs::remove_dir_all(&dir).ok();
    }
}
