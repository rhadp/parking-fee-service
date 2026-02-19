//! Configuration for the UPDATE_SERVICE.
//!
//! The [`Config`] struct holds all runtime configuration, parsed from
//! command-line arguments and environment variables using `clap`.
//!
//! # Defaults
//!
//! | Parameter | Default | Env Var |
//! |-----------|---------|---------|
//! | `listen_addr` | `0.0.0.0:50053` | `LISTEN_ADDR` |
//! | `data_dir` | `./data` | `DATA_DIR` |
//! | `offload_timeout` | `5m` | `OFFLOAD_TIMEOUT` |
//!
//! # Requirements
//!
//! - 04-REQ-4.6: Accept configuration via env vars or CLI flags.
//! - 04-REQ-5.4: Offloading timeout is configurable.

use std::time::Duration;

use clap::Parser;

/// RHIVOS Update Service configuration.
///
/// Parsed from CLI arguments and environment variables.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "update-service",
    about = "RHIVOS update service — manages adapter container lifecycle"
)]
pub struct Config {
    /// Address to listen on for gRPC requests.
    #[arg(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:50053")]
    pub listen_addr: String,

    /// Directory for persisting adapter state.
    #[arg(long, env = "DATA_DIR", default_value = "./data")]
    pub data_dir: String,

    /// Offloading timeout for unused adapters.
    ///
    /// Adapters are removed after this period of inactivity.
    /// Supports formats like `5m`, `30s`, `1h`, `300s`.
    #[arg(long, env = "OFFLOAD_TIMEOUT", default_value = "5m")]
    pub offload_timeout: String,
}

impl Config {
    /// Parse the offload timeout string into a [`Duration`].
    ///
    /// Supported formats:
    /// - `<n>s` — seconds (e.g. `300s`)
    /// - `<n>m` — minutes (e.g. `5m`)
    /// - `<n>h` — hours (e.g. `1h`)
    /// - `<n>` — plain number treated as seconds
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed.
    pub fn offload_duration(&self) -> Result<Duration, String> {
        parse_duration(&self.offload_timeout)
    }
}

/// Parse a human-friendly duration string into a [`Duration`].
fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty duration string".to_string());
    }

    if let Some(rest) = s.strip_suffix('h') {
        let n: u64 = rest
            .parse()
            .map_err(|e| format!("invalid hours value '{}': {}", rest, e))?;
        Ok(Duration::from_secs(n * 3600))
    } else if let Some(rest) = s.strip_suffix('m') {
        let n: u64 = rest
            .parse()
            .map_err(|e| format!("invalid minutes value '{}': {}", rest, e))?;
        Ok(Duration::from_secs(n * 60))
    } else if let Some(rest) = s.strip_suffix('s') {
        let n: u64 = rest
            .parse()
            .map_err(|e| format!("invalid seconds value '{}': {}", rest, e))?;
        Ok(Duration::from_secs(n))
    } else {
        // Plain number → seconds
        let n: u64 = s
            .parse()
            .map_err(|e| format!("invalid duration '{}': {}", s, e))?;
        Ok(Duration::from_secs(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = Config::parse_from(["update-service"]);
        assert_eq!(config.listen_addr, "0.0.0.0:50053");
        assert_eq!(config.data_dir, "./data");
        assert_eq!(config.offload_timeout, "5m");
    }

    #[test]
    fn custom_listen_addr() {
        let config = Config::parse_from([
            "update-service",
            "--listen-addr",
            "127.0.0.1:9999",
        ]);
        assert_eq!(config.listen_addr, "127.0.0.1:9999");
    }

    #[test]
    fn custom_data_dir() {
        let config = Config::parse_from([
            "update-service",
            "--data-dir",
            "/var/lib/update-service",
        ]);
        assert_eq!(config.data_dir, "/var/lib/update-service");
    }

    #[test]
    fn custom_offload_timeout() {
        let config = Config::parse_from([
            "update-service",
            "--offload-timeout",
            "10m",
        ]);
        assert_eq!(config.offload_timeout, "10m");
    }

    #[test]
    fn all_custom_args() {
        let config = Config::parse_from([
            "update-service",
            "--listen-addr",
            "10.0.0.1:50053",
            "--data-dir",
            "/tmp/data",
            "--offload-timeout",
            "300s",
        ]);
        assert_eq!(config.listen_addr, "10.0.0.1:50053");
        assert_eq!(config.data_dir, "/tmp/data");
        assert_eq!(config.offload_timeout, "300s");
    }

    #[test]
    fn config_is_clone() {
        let config = Config::parse_from(["update-service"]);
        let cloned = config.clone();
        assert_eq!(cloned.listen_addr, config.listen_addr);
        assert_eq!(cloned.data_dir, config.data_dir);
        assert_eq!(cloned.offload_timeout, config.offload_timeout);
    }

    #[test]
    fn config_is_debug() {
        let config = Config::parse_from(["update-service"]);
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("listen_addr"));
        assert!(debug_str.contains("data_dir"));
        assert!(debug_str.contains("offload_timeout"));
    }

    // ---- parse_duration tests ----

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("1m").unwrap(), Duration::from_secs(60));
        assert_eq!(parse_duration("0m").unwrap(), Duration::from_secs(0));
    }

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(parse_duration("300s").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("0s").unwrap(), Duration::from_secs(0));
    }

    #[test]
    fn parse_duration_hours() {
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn parse_duration_plain_number() {
        assert_eq!(parse_duration("300").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("60").unwrap(), Duration::from_secs(60));
    }

    #[test]
    fn parse_duration_invalid() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("5x").is_err());
        assert!(parse_duration("-1m").is_err());
    }

    #[test]
    fn offload_duration_default() {
        let config = Config::parse_from(["update-service"]);
        let duration = config.offload_duration().unwrap();
        assert_eq!(duration, Duration::from_secs(300));
    }

    #[test]
    fn offload_duration_custom() {
        let config = Config::parse_from([
            "update-service",
            "--offload-timeout",
            "30s",
        ]);
        let duration = config.offload_duration().unwrap();
        assert_eq!(duration, Duration::from_secs(30));
    }
}
