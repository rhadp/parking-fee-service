use serde::Deserialize;

/// UPDATE_SERVICE configuration.
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
            registry_base_url: "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters"
                .to_string(),
            inactivity_timeout_secs: 86400,
            storage_path: "/var/lib/containers/adapters/".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from an optional TOML file path, with env var overrides.
    pub fn load(_path: Option<&str>) -> Result<Self, String> {
        // Stub: returns defaults for now
        Ok(Self::default())
    }
}
