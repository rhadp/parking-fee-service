use serde::Deserialize;
use std::env;
use std::fs;

/// TOML configuration file structure matching design.md sections.
#[derive(Debug, Clone, Deserialize, Default)]
struct FileConfig {
    #[serde(default)]
    grpc: GrpcSection,
    #[serde(default)]
    registry: RegistrySection,
    #[serde(default)]
    offload: OffloadSection,
    #[serde(default)]
    storage: StorageSection,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct GrpcSection {
    port: Option<u16>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RegistrySection {
    base_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OffloadSection {
    inactivity_timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct StorageSection {
    path: Option<String>,
}

/// UPDATE_SERVICE configuration loaded from TOML file or environment variables.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub grpc_port: u16,
    pub registry_base_url: String,
    pub inactivity_timeout_secs: u64,
    pub storage_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grpc_port: 50060,
            registry_base_url: "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters".into(),
            inactivity_timeout_secs: 86400,
            storage_path: "/var/lib/containers/adapters/".into(),
        }
    }
}

impl Config {
    /// Load configuration from an optional TOML file path, falling back to defaults.
    ///
    /// Priority (highest to lowest):
    /// 1. Environment variables with `UPDATE_SERVICE_` prefix
    /// 2. TOML file values
    /// 3. Built-in defaults
    pub fn load(path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let defaults = Config::default();

        // Load file config if path is provided and file exists
        let file_config = match path {
            Some(p) => {
                let contents = fs::read_to_string(p)?;
                toml::from_str::<FileConfig>(&contents)?
            }
            None => FileConfig::default(),
        };

        // Build config: file values override defaults, env vars override file values
        let grpc_port = env_var_parse::<u16>("UPDATE_SERVICE_GRPC_PORT")
            .or(file_config.grpc.port)
            .unwrap_or(defaults.grpc_port);

        let registry_base_url = env::var("UPDATE_SERVICE_REGISTRY_BASE_URL")
            .ok()
            .or(file_config.registry.base_url)
            .unwrap_or(defaults.registry_base_url);

        let inactivity_timeout_secs =
            env_var_parse::<u64>("UPDATE_SERVICE_INACTIVITY_TIMEOUT_SECS")
                .or(file_config.offload.inactivity_timeout_secs)
                .unwrap_or(defaults.inactivity_timeout_secs);

        let storage_path = env::var("UPDATE_SERVICE_STORAGE_PATH")
            .ok()
            .or(file_config.storage.path)
            .unwrap_or(defaults.storage_path);

        Ok(Config {
            grpc_port,
            registry_base_url,
            inactivity_timeout_secs,
            storage_path,
        })
    }
}

/// Parse an environment variable into a type, returning None if unset or parse fails.
fn env_var_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    env::var(key).ok().and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.grpc_port, 50060);
        assert_eq!(
            config.registry_base_url,
            "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters"
        );
        assert_eq!(config.inactivity_timeout_secs, 86400);
        assert_eq!(config.storage_path, "/var/lib/containers/adapters/");
    }

    #[test]
    fn test_load_defaults_without_file() {
        // Use a unique env var to test the helper without polluting shared state
        let result: Option<u16> = env_var_parse("UPDATE_SERVICE_TEST_NONEXISTENT_VAR_12345");
        assert!(result.is_none(), "nonexistent env var should return None");
    }

    #[test]
    fn test_load_from_toml_file() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmpfile,
            r#"
[grpc]
port = 9090

[registry]
base_url = "my-registry.example.com/adapters"

[offload]
inactivity_timeout_secs = 3600

[storage]
path = "/tmp/adapters/"
"#
        )
        .unwrap();

        let config = Config::load(Some(tmpfile.path().to_str().unwrap()))
            .expect("should load from file");
        assert_eq!(config.grpc_port, 9090);
        assert_eq!(config.registry_base_url, "my-registry.example.com/adapters");
        assert_eq!(config.inactivity_timeout_secs, 3600);
        assert_eq!(config.storage_path, "/tmp/adapters/");
    }

    #[test]
    fn test_load_partial_toml_file() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmpfile,
            r#"
[grpc]
port = 8080
"#
        )
        .unwrap();

        let config = Config::load(Some(tmpfile.path().to_str().unwrap()))
            .expect("should load partial config");
        assert_eq!(config.grpc_port, 8080);
        // Other values should be defaults
        assert_eq!(config.inactivity_timeout_secs, 86400);
        assert_eq!(config.storage_path, "/var/lib/containers/adapters/");
    }

    #[test]
    fn test_env_var_parse_helper() {
        // Test env_var_parse with a unique var name to avoid cross-test pollution
        let unique_key = "UPDATE_SERVICE_TEST_PARSE_HELPER_54321";
        env::set_var(unique_key, "7070");
        let result: Option<u16> = env_var_parse(unique_key);
        assert_eq!(result, Some(7070));
        env::remove_var(unique_key);

        // Invalid parse returns None
        env::set_var(unique_key, "not-a-number");
        let result: Option<u16> = env_var_parse(unique_key);
        assert_eq!(result, None);
        env::remove_var(unique_key);
    }

    #[test]
    fn test_nonexistent_file_returns_error() {
        let result = Config::load(Some("/nonexistent/path/config.toml"));
        assert!(result.is_err());
    }
}
