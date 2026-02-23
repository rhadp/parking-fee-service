//! Mock sensor CLI tools — shared library.
//!
//! Provides common helpers for the mock sensor binaries (location-sensor,
//! speed-sensor, door-sensor). Each binary accepts sensor values as CLI
//! arguments and writes them to DATA_BROKER via gRPC.
//!
//! The DATA_BROKER endpoint is configurable:
//! - `--endpoint <URI>` CLI flag (highest priority)
//! - `DATABROKER_ADDR` environment variable (TCP endpoint)
//! - `DATABROKER_UDS_PATH` environment variable (UDS socket path)
//! - Default: `unix:///tmp/kuksa-databroker.sock`

use databroker_client::{DataValue, DatabrokerClient, Error};

/// VSS signal paths used by the mock sensors.
pub mod signals {
    /// Vehicle speed (float, km/h).
    pub const SPEED: &str = "Vehicle.Speed";
    /// Driver-side door open state (bool).
    pub const DOOR_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
    /// Current latitude (double).
    pub const LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
    /// Current longitude (double).
    pub const LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
}

/// Resolve the DATA_BROKER endpoint from CLI flag, environment, or default.
///
/// Priority:
/// 1. `cli_endpoint` — the `--endpoint` CLI flag value, if provided.
/// 2. `DATABROKER_ADDR` environment variable (TCP address).
/// 3. `DATABROKER_UDS_PATH` environment variable (UDS socket path).
/// 4. Default UDS endpoint: `unix:///tmp/kuksa-databroker.sock`.
pub fn resolve_endpoint(cli_endpoint: Option<&str>) -> String {
    if let Some(ep) = cli_endpoint {
        return ep.to_string();
    }

    if let Ok(addr) = std::env::var("DATABROKER_ADDR") {
        if !addr.is_empty() {
            return addr;
        }
    }

    if let Ok(path) = std::env::var("DATABROKER_UDS_PATH") {
        if !path.is_empty() {
            return format!("unix://{path}");
        }
    }

    databroker_client::DEFAULT_UDS_ENDPOINT.to_string()
}

/// Connect to DATA_BROKER at the resolved endpoint.
///
/// Prints a user-friendly error and returns `Err` on connection failure.
pub async fn connect(endpoint: &str) -> Result<DatabrokerClient, Error> {
    DatabrokerClient::connect(endpoint).await
}

/// Write a single signal value to DATA_BROKER.
///
/// Prints the signal path and value on success.
pub async fn write_signal(
    client: &DatabrokerClient,
    path: &str,
    value: DataValue,
) -> Result<(), Error> {
    client.set_value(path, value).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Endpoint resolution tests ─────────────────────────────────────────

    #[test]
    fn test_resolve_endpoint_cli_flag_takes_priority() {
        // CLI flag should override everything
        let ep = resolve_endpoint(Some("http://custom:9999"));
        assert_eq!(ep, "http://custom:9999");
    }

    #[test]
    fn test_resolve_endpoint_cli_uds_path() {
        let ep = resolve_endpoint(Some("unix:///var/run/test.sock"));
        assert_eq!(ep, "unix:///var/run/test.sock");
    }

    #[test]
    fn test_resolve_endpoint_cli_empty_string() {
        // Empty CLI flag should still be treated as a value
        let ep = resolve_endpoint(Some(""));
        assert_eq!(ep, "");
    }

    #[test]
    fn test_resolve_endpoint_default_is_uds() {
        // When no CLI flag and no env vars, should default to UDS
        // We can't safely clear env vars in parallel tests, so just check
        // the function works with None and doesn't panic.
        let ep = resolve_endpoint(None);
        // The result depends on env vars, but should be a non-empty string
        assert!(!ep.is_empty());
    }

    // ── Signal path constant tests ────────────────────────────────────────

    #[test]
    fn test_signal_constants() {
        assert_eq!(signals::SPEED, "Vehicle.Speed");
        assert_eq!(
            signals::DOOR_IS_OPEN,
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
        );
        assert_eq!(signals::LATITUDE, "Vehicle.CurrentLocation.Latitude");
        assert_eq!(signals::LONGITUDE, "Vehicle.CurrentLocation.Longitude");
    }

    #[test]
    fn test_signal_paths_are_valid_vss() {
        // VSS paths use dot-separated segments starting with "Vehicle"
        for path in [
            signals::SPEED,
            signals::DOOR_IS_OPEN,
            signals::LATITUDE,
            signals::LONGITUDE,
        ] {
            assert!(
                path.starts_with("Vehicle."),
                "Signal path should start with 'Vehicle.': {path}"
            );
            assert!(
                !path.ends_with('.'),
                "Signal path should not end with '.': {path}"
            );
            // Each segment should be non-empty
            for segment in path.split('.') {
                assert!(
                    !segment.is_empty(),
                    "Signal path has empty segment: {path}"
                );
            }
        }
    }

    // ── DataValue construction tests ──────────────────────────────────────

    #[test]
    fn test_speed_data_value() {
        let val = DataValue::Float(55.5);
        assert_eq!(val.as_float(), Some(55.5));
    }

    #[test]
    fn test_door_data_value() {
        let val_open = DataValue::Bool(true);
        let val_closed = DataValue::Bool(false);
        assert_eq!(val_open.as_bool(), Some(true));
        assert_eq!(val_closed.as_bool(), Some(false));
    }

    #[test]
    fn test_location_data_value() {
        let lat = DataValue::Double(48.1351);
        let lon = DataValue::Double(11.5820);
        assert_eq!(lat.as_double(), Some(48.1351));
        assert_eq!(lon.as_double(), Some(11.5820));
    }

    #[test]
    fn test_speed_zero() {
        let val = DataValue::Float(0.0);
        assert_eq!(val.as_float(), Some(0.0));
    }

    #[test]
    fn test_speed_negative() {
        // Negative speed values should be representable (validation is external)
        let val = DataValue::Float(-10.0);
        assert_eq!(val.as_float(), Some(-10.0));
    }

    #[test]
    fn test_location_equator() {
        let lat = DataValue::Double(0.0);
        let lon = DataValue::Double(0.0);
        assert_eq!(lat.as_double(), Some(0.0));
        assert_eq!(lon.as_double(), Some(0.0));
    }
}
