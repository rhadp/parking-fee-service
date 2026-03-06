use serde::Deserialize;

/// UPDATE_SERVICE configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    #[serde(default = "default_registry_base_url")]
    pub registry_base_url: String,
    #[serde(default = "default_inactivity_timeout_secs")]
    pub inactivity_timeout_secs: u64,
    #[serde(default = "default_storage_path")]
    pub storage_path: String,
}

fn default_grpc_port() -> u16 {
    50060
}

fn default_registry_base_url() -> String {
    "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters".to_string()
}

fn default_inactivity_timeout_secs() -> u64 {
    86400
}

fn default_storage_path() -> String {
    "/var/lib/containers/adapters/".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grpc_port: default_grpc_port(),
            registry_base_url: default_registry_base_url(),
            inactivity_timeout_secs: default_inactivity_timeout_secs(),
            storage_path: default_storage_path(),
        }
    }
}

impl Config {
    /// Load configuration from an optional TOML file path, with environment
    /// variable overrides using the `UPDATE_SERVICE_` prefix.
    ///
    /// Priority: env vars > TOML file values > defaults.
    ///
    /// Supported env vars:
    /// - `UPDATE_SERVICE_GRPC_PORT`
    /// - `UPDATE_SERVICE_REGISTRY_BASE_URL`
    /// - `UPDATE_SERVICE_INACTIVITY_TIMEOUT_SECS`
    /// - `UPDATE_SERVICE_STORAGE_PATH`
    pub fn load(path: Option<&str>) -> Result<Self, String> {
        // Start from file or defaults.
        let mut config = if let Some(file_path) = path {
            let contents = std::fs::read_to_string(file_path)
                .map_err(|e| format!("failed to read config file '{file_path}': {e}"))?;
            toml::from_str::<Config>(&contents)
                .map_err(|e| format!("failed to parse config file '{file_path}': {e}"))?
        } else {
            Config::default()
        };

        // Apply environment variable overrides.
        if let Ok(val) = std::env::var("UPDATE_SERVICE_GRPC_PORT") {
            config.grpc_port = val
                .parse()
                .map_err(|e| format!("invalid UPDATE_SERVICE_GRPC_PORT: {e}"))?;
        }
        if let Ok(val) = std::env::var("UPDATE_SERVICE_REGISTRY_BASE_URL") {
            config.registry_base_url = val;
        }
        if let Ok(val) = std::env::var("UPDATE_SERVICE_INACTIVITY_TIMEOUT_SECS") {
            config.inactivity_timeout_secs = val
                .parse()
                .map_err(|e| format!("invalid UPDATE_SERVICE_INACTIVITY_TIMEOUT_SECS: {e}"))?;
        }
        if let Ok(val) = std::env::var("UPDATE_SERVICE_STORAGE_PATH") {
            config.storage_path = val;
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.grpc_port, 50060);
        assert_eq!(config.inactivity_timeout_secs, 86400);
        assert_eq!(config.storage_path, "/var/lib/containers/adapters/");
        assert!(config.registry_base_url.contains("adapters"));
    }

    #[test]
    fn test_load_defaults_when_no_file() {
        let config = Config::load(None).unwrap();
        assert_eq!(config.grpc_port, 50060);
        assert_eq!(config.inactivity_timeout_secs, 86400);
    }

    #[test]
    fn test_load_from_toml_file() {
        // Clear env vars so they don't override TOML values.
        std::env::remove_var("UPDATE_SERVICE_GRPC_PORT");
        std::env::remove_var("UPDATE_SERVICE_REGISTRY_BASE_URL");
        std::env::remove_var("UPDATE_SERVICE_INACTIVITY_TIMEOUT_SECS");
        std::env::remove_var("UPDATE_SERVICE_STORAGE_PATH");

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            r#"
grpc_port = 9999
registry_base_url = "example.com/repo"
inactivity_timeout_secs = 3600
storage_path = "/tmp/adapters"
"#
        )
        .unwrap();

        let config = Config::load(Some(tmp.path().to_str().unwrap())).unwrap();
        assert_eq!(config.grpc_port, 9999);
        assert_eq!(config.registry_base_url, "example.com/repo");
        assert_eq!(config.inactivity_timeout_secs, 3600);
        assert_eq!(config.storage_path, "/tmp/adapters");
    }

    #[test]
    fn test_load_partial_toml_uses_defaults() {
        // Clear env vars so they don't override TOML values.
        std::env::remove_var("UPDATE_SERVICE_GRPC_PORT");
        std::env::remove_var("UPDATE_SERVICE_REGISTRY_BASE_URL");
        std::env::remove_var("UPDATE_SERVICE_INACTIVITY_TIMEOUT_SECS");
        std::env::remove_var("UPDATE_SERVICE_STORAGE_PATH");

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, r#"grpc_port = 12345"#).unwrap();

        let config = Config::load(Some(tmp.path().to_str().unwrap())).unwrap();
        assert_eq!(config.grpc_port, 12345);
        // Remaining fields use defaults
        assert_eq!(config.inactivity_timeout_secs, 86400);
        assert_eq!(config.storage_path, "/var/lib/containers/adapters/");
    }

    #[test]
    fn test_load_missing_file_returns_error() {
        let result = Config::load(Some("/nonexistent/path/config.toml"));
        assert!(result.is_err());
    }
}
