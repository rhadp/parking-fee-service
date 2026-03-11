/// Configuration for the LOCKING_SERVICE.
pub struct Config {
    /// Unix Domain Socket path for DATA_BROKER gRPC connection.
    pub databroker_uds_path: String,
}

impl Config {
    /// Default UDS path for DATA_BROKER.
    pub const DEFAULT_UDS_PATH: &'static str = "/tmp/kuksa/databroker.sock";

    /// Parse configuration from environment variables.
    pub fn from_env() -> Self {
        let databroker_uds_path = std::env::var("DATABROKER_UDS_PATH")
            .unwrap_or_else(|_| Self::DEFAULT_UDS_PATH.to_string());
        Config { databroker_uds_path }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_uds_path_when_env_unset() {
        // TS-03-1 precondition: DATABROKER_UDS_PATH defaults to /tmp/kuksa/databroker.sock
        // Clear the env var to test default behavior
        std::env::remove_var("DATABROKER_UDS_PATH");
        let config = Config::from_env();
        assert_eq!(
            config.databroker_uds_path,
            "/tmp/kuksa/databroker.sock",
            "Default UDS path should be /tmp/kuksa/databroker.sock when env var is unset"
        );
    }

    #[test]
    fn test_uds_path_parsed_from_env() {
        // TS-03-1 precondition: DATABROKER_UDS_PATH is parsed from environment when set
        let custom_path = "/custom/path/databroker.sock";
        std::env::set_var("DATABROKER_UDS_PATH", custom_path);
        let config = Config::from_env();
        std::env::remove_var("DATABROKER_UDS_PATH");
        assert_eq!(
            config.databroker_uds_path, custom_path,
            "UDS path should be parsed from DATABROKER_UDS_PATH environment variable"
        );
    }
}
