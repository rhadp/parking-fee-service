//! VIN generation, persistence, and pairing PIN management.
//!
//! On first startup, CLOUD_GATEWAY_CLIENT generates a random VIN and pairing
//! PIN, persists them to `{data_dir}/vin.json`, and logs them to stdout.
//! On subsequent starts the persisted values are reused.
//!
//! # VIN Format
//!
//! `DEMO` + 13 random alphanumeric characters (17 chars total — standard VIN
//! length).
//!
//! # Pairing PIN Format
//!
//! 6-digit numeric string (zero-padded), e.g. `"048291"`.
//!
//! # Requirements
//!
//! - 03-REQ-5.1: Generate VIN and PIN, persist to data directory, log to stdout.
//! - 03-REQ-5.E3: Reuse persisted VIN and PIN on subsequent starts.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

/// Filename used for VIN persistence inside the data directory.
const VIN_FILENAME: &str = "vin.json";

/// Persisted VIN and pairing PIN.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VinData {
    /// Vehicle Identification Number (17 chars, starts with "DEMO").
    pub vin: String,
    /// 6-digit numeric pairing PIN.
    pub pairing_pin: String,
}

/// Generate a random VIN: `DEMO` + 13 random alphanumeric characters.
///
/// The result is always 17 characters long (standard VIN length).
pub fn generate_vin() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const SUFFIX_LEN: usize = 13;

    let mut rng = rand::thread_rng();
    let suffix: String = (0..SUFFIX_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    format!("DEMO{suffix}")
}

/// Generate a random 6-digit numeric pairing PIN (zero-padded).
pub fn generate_pairing_pin() -> String {
    let pin: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{pin:06}")
}

/// Load existing VIN data from `{data_dir}/vin.json`, or generate and persist
/// new data if the file does not exist.
///
/// # Errors
///
/// Returns an error if the data directory cannot be created, or if reading /
/// writing the VIN file fails.
pub fn load_or_create(data_dir: &Path) -> Result<VinData, VinError> {
    let vin_path = vin_file_path(data_dir);

    if vin_path.exists() {
        // Read existing data.
        let contents = std::fs::read_to_string(&vin_path).map_err(|e| VinError::Io {
            path: vin_path.clone(),
            source: e,
        })?;
        let data: VinData =
            serde_json::from_str(&contents).map_err(|e| VinError::Parse {
                path: vin_path.clone(),
                source: e,
            })?;
        info!(vin = %data.vin, "loaded existing VIN from {}", vin_path.display());
        Ok(data)
    } else {
        // Generate new VIN and PIN.
        let data = VinData {
            vin: generate_vin(),
            pairing_pin: generate_pairing_pin(),
        };

        // Ensure the data directory exists.
        std::fs::create_dir_all(data_dir).map_err(|e| VinError::Io {
            path: data_dir.to_path_buf(),
            source: e,
        })?;

        // Persist to disk.
        let json = serde_json::to_string_pretty(&data).expect("VinData serializes to JSON");
        std::fs::write(&vin_path, json).map_err(|e| VinError::Io {
            path: vin_path.clone(),
            source: e,
        })?;

        info!(
            vin = %data.vin,
            pairing_pin = %data.pairing_pin,
            "generated new VIN and pairing PIN, persisted to {}",
            vin_path.display()
        );
        Ok(data)
    }
}

/// Returns the full path to the VIN persistence file.
fn vin_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join(VIN_FILENAME)
}

/// Errors that can occur during VIN loading or generation.
#[derive(Debug)]
pub enum VinError {
    /// I/O error reading or writing the VIN file or data directory.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// JSON parse error when reading an existing VIN file.
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl std::fmt::Display for VinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VinError::Io { path, source } => {
                write!(f, "VIN I/O error at {}: {}", path.display(), source)
            }
            VinError::Parse { path, source } => {
                write!(f, "VIN parse error in {}: {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for VinError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VinError::Io { source, .. } => Some(source),
            VinError::Parse { source, .. } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn generate_vin_format() {
        let vin = generate_vin();
        assert_eq!(vin.len(), 17, "VIN must be 17 characters");
        assert!(vin.starts_with("DEMO"), "VIN must start with DEMO");
        // Remaining 13 chars must be alphanumeric uppercase + digits.
        assert!(
            vin[4..].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
            "VIN suffix must be alphanumeric: {vin}"
        );
    }

    #[test]
    fn generate_vin_uniqueness() {
        let vins: Vec<String> = (0..100).map(|_| generate_vin()).collect();
        let unique: std::collections::HashSet<&String> = vins.iter().collect();
        // With 36^13 possible values, collisions in 100 samples are practically impossible.
        assert_eq!(unique.len(), 100, "VINs should be unique");
    }

    #[test]
    fn generate_pairing_pin_format() {
        let pin = generate_pairing_pin();
        assert_eq!(pin.len(), 6, "PIN must be 6 characters");
        assert!(
            pin.chars().all(|c| c.is_ascii_digit()),
            "PIN must be all digits: {pin}"
        );
    }

    #[test]
    fn generate_pairing_pin_zero_padded() {
        // Run many times to increase chance of hitting a small number.
        for _ in 0..1000 {
            let pin = generate_pairing_pin();
            assert_eq!(pin.len(), 6, "PIN must always be 6 digits: {pin}");
        }
    }

    #[test]
    fn load_or_create_generates_new() {
        let dir = TempDir::new().unwrap();
        let data = load_or_create(dir.path()).unwrap();

        assert_eq!(data.vin.len(), 17);
        assert!(data.vin.starts_with("DEMO"));
        assert_eq!(data.pairing_pin.len(), 6);

        // File should exist now.
        let vin_path = dir.path().join("vin.json");
        assert!(vin_path.exists(), "vin.json should be created");

        // Content should deserialize back to the same data.
        let contents = std::fs::read_to_string(&vin_path).unwrap();
        let loaded: VinData = serde_json::from_str(&contents).unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn load_or_create_reuses_existing() {
        let dir = TempDir::new().unwrap();

        // First call generates.
        let data1 = load_or_create(dir.path()).unwrap();
        // Second call reuses.
        let data2 = load_or_create(dir.path()).unwrap();

        assert_eq!(data1, data2, "VIN and PIN should be reused across restarts");
    }

    #[test]
    fn load_or_create_creates_nested_dir() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("a").join("b").join("c");

        let data = load_or_create(&nested).unwrap();
        assert_eq!(data.vin.len(), 17);
        assert!(nested.join("vin.json").exists());
    }

    #[test]
    fn load_or_create_invalid_json() {
        let dir = TempDir::new().unwrap();
        let vin_path = dir.path().join("vin.json");
        std::fs::write(&vin_path, "not valid json").unwrap();

        let result = load_or_create(dir.path());
        assert!(result.is_err(), "Should fail on invalid JSON");
        let err = result.unwrap_err();
        assert!(
            matches!(err, VinError::Parse { .. }),
            "Should be a parse error"
        );
    }

    #[test]
    fn vin_data_json_format() {
        let data = VinData {
            vin: "DEMO0000000000001".to_string(),
            pairing_pin: "482916".to_string(),
        };
        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["vin"], "DEMO0000000000001");
        assert_eq!(json["pairing_pin"], "482916");
        assert_eq!(json.as_object().unwrap().len(), 2);
    }

    #[test]
    fn vin_data_roundtrip() {
        let data = VinData {
            vin: "DEMO0000000000001".to_string(),
            pairing_pin: "048291".to_string(),
        };
        let json_str = serde_json::to_string(&data).unwrap();
        let deserialized: VinData = serde_json::from_str(&json_str).unwrap();
        assert_eq!(data, deserialized);
    }

    #[test]
    fn vin_error_display() {
        let err = VinError::Io {
            path: PathBuf::from("/tmp/test"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = format!("{err}");
        assert!(msg.contains("/tmp/test"));
        assert!(msg.contains("not found"));
    }
}
