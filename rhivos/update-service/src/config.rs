use serde::Deserialize;

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
    pub fn load(_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        // Stub: returns defaults. Implementation in task group 2.
        Ok(Self::default())
    }
}
