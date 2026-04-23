use serde::Deserialize;

/// Service configuration loaded from a JSON file.
///
/// Each field has a built-in default (per 07-REQ-7.2) so the config file
/// may omit any subset of fields — missing ones are filled from defaults.
///
/// Note: `registry_url` and `container_storage_path` are specified by
/// 07-REQ-7.2 but are not consumed by any current execution path. They
/// exist for forward-compatibility; see `docs/errata/07_inert_config_fields.md`.
#[derive(Debug, Clone, Deserialize)]
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
pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let cfg: Config = serde_json::from_str(&contents).map_err(ConfigError::Parse)?;
            Ok(cfg)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("config file not found at {path}, using defaults");
            Ok(default_config())
        }
        Err(e) => Err(ConfigError::Io(e)),
    }
}

/// Returns the built-in default configuration.
pub fn default_config() -> Config {
    Config {
        grpc_port: 50052,
        registry_url: String::new(),
        inactivity_timeout_secs: 86400,
        container_storage_path: "/var/lib/containers/adapters/".to_string(),
    }
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

    // Partial config: missing fields use defaults (REQ-7.2 per-field defaults)
    #[test]
    fn test_load_config_partial_fields() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_partial_config_07.json");
        std::fs::write(&path, r#"{"grpc_port": 60000}"#).unwrap();

        let cfg = load_config(path.to_str().unwrap()).unwrap();
        assert_eq!(cfg.grpc_port, 60000);
        // Missing fields should use defaults
        assert_eq!(cfg.registry_url, "");
        assert_eq!(cfg.inactivity_timeout_secs, 86400);
        assert_eq!(cfg.container_storage_path, "/var/lib/containers/adapters/");

        let _ = std::fs::remove_file(&path);
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
