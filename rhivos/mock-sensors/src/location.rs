//! Location sensor argument parsing.
//!
//! Provides helpers for parsing `--lat` and `--lon` CLI arguments
//! used by the `location-sensor` binary.

/// VSS signal path for vehicle latitude.
pub const LAT_SIGNAL: &str = crate::LOCATION_LAT_SIGNAL;

/// VSS signal path for vehicle longitude.
pub const LON_SIGNAL: &str = crate::LOCATION_LON_SIGNAL;

/// Parsed location sensor arguments.
#[derive(Debug, Clone)]
pub struct LocationArgs {
    /// Latitude value (f64/double).
    pub lat: f64,
    /// Longitude value (f64/double).
    pub lon: f64,
    /// DATA_BROKER address.
    pub broker_addr: String,
}

/// Parse location sensor CLI arguments.
///
/// Expects `--lat=<value>` and `--lon=<value>`. Optionally accepts
/// `--broker-addr=<addr>`. Returns `None` for `--help`/`-h`.
///
/// # Errors
///
/// Returns an error string if required arguments are missing or invalid.
pub fn parse_args(args: &[String]) -> Result<Option<LocationArgs>, String> {
    let mut lat: Option<f64> = None;
    let mut lon: Option<f64> = None;
    let mut broker_addr: Option<String> = None;

    for arg in args.iter().skip(1) {
        if arg == "--help" || arg == "-h" {
            return Ok(None);
        } else if let Some(val) = arg.strip_prefix("--lat=") {
            lat = Some(
                val.parse::<f64>()
                    .map_err(|e| format!("invalid --lat value: {}", e))?,
            );
        } else if let Some(val) = arg.strip_prefix("--lon=") {
            lon = Some(
                val.parse::<f64>()
                    .map_err(|e| format!("invalid --lon value: {}", e))?,
            );
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = Some(val.to_string());
        } else {
            return Err(format!("unknown argument: {}", arg));
        }
    }

    let lat = lat.ok_or_else(|| "required argument --lat is missing".to_string())?;
    let lon = lon.ok_or_else(|| "required argument --lon is missing".to_string())?;

    let addr = broker_addr
        .or_else(|| std::env::var("DATA_BROKER_ADDR").ok())
        .unwrap_or_else(|| crate::DEFAULT_BROKER_ADDR.to_string());

    Ok(Some(LocationArgs {
        lat,
        lon,
        broker_addr: addr,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_args() {
        let args = vec![
            "location-sensor".to_string(),
            "--lat=48.1351".to_string(),
            "--lon=11.5820".to_string(),
        ];
        std::env::remove_var("DATA_BROKER_ADDR");
        let result = parse_args(&args).unwrap().unwrap();
        assert!((result.lat - 48.1351).abs() < f64::EPSILON);
        assert!((result.lon - 11.5820).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_missing_lat() {
        let args = vec!["location-sensor".to_string(), "--lon=11.0".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_help() {
        let args = vec!["location-sensor".to_string(), "--help".to_string()];
        assert!(parse_args(&args).unwrap().is_none());
    }
}
