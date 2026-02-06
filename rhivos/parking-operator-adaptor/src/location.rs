//! Location types and location reading from DATA_BROKER.
//!
//! This module provides the Location struct and LocationReader for
//! fetching vehicle location from DATA_BROKER.

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::ParkingError;

/// Vehicle location with latitude and longitude.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    /// Latitude in degrees (-90 to 90)
    pub latitude: f64,
    /// Longitude in degrees (-180 to 180)
    pub longitude: f64,
}

impl Location {
    /// Create a new location.
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    /// Check if the location is valid (within valid coordinate ranges).
    pub fn is_valid(&self) -> bool {
        (-90.0..=90.0).contains(&self.latitude) && (-180.0..=180.0).contains(&self.longitude)
    }
}

impl Default for Location {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
        }
    }
}

/// Reads vehicle location from DATA_BROKER.
#[derive(Clone)]
pub struct LocationReader {
    /// DATA_BROKER socket path
    data_broker_socket: String,
    /// Mock location for testing
    #[cfg(test)]
    mock_location: std::sync::Arc<tokio::sync::RwLock<Option<Location>>>,
}

impl LocationReader {
    /// Create a new LocationReader.
    pub fn new(data_broker_socket: String) -> Self {
        Self {
            data_broker_socket,
            #[cfg(test)]
            mock_location: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Read the current vehicle location from DATA_BROKER.
    ///
    /// Returns error if location signals are unavailable.
    pub async fn read_location(&self) -> Result<Location, ParkingError> {
        #[cfg(test)]
        {
            if let Some(loc) = self.mock_location.read().await.clone() {
                return Ok(loc);
            }
        }

        debug!(
            "Reading location from DATA_BROKER at {}",
            self.data_broker_socket
        );

        // In a real implementation, this would:
        // 1. Connect to DATA_BROKER via gRPC/UDS
        // 2. Read Vehicle.CurrentLocation.Latitude
        // 3. Read Vehicle.CurrentLocation.Longitude
        // 4. Return Location or error if unavailable

        // For now, return a placeholder that indicates location is unavailable
        // This will be replaced with actual DATA_BROKER integration
        Err(ParkingError::LocationUnavailable(
            "DATA_BROKER integration pending".to_string(),
        ))
    }

    /// Set mock location for testing.
    #[cfg(test)]
    pub async fn set_mock_location(&self, location: Option<Location>) {
        *self.mock_location.write().await = location;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_location_new() {
        let loc = Location::new(37.7749, -122.4194);
        assert!((loc.latitude - 37.7749).abs() < 0.0001);
        assert!((loc.longitude - (-122.4194)).abs() < 0.0001);
    }

    #[test]
    fn test_location_is_valid() {
        assert!(Location::new(0.0, 0.0).is_valid());
        assert!(Location::new(90.0, 180.0).is_valid());
        assert!(Location::new(-90.0, -180.0).is_valid());
        assert!(!Location::new(91.0, 0.0).is_valid());
        assert!(!Location::new(0.0, 181.0).is_valid());
    }

    #[test]
    fn test_location_default() {
        let loc = Location::default();
        assert!((loc.latitude).abs() < 0.0001);
        assert!((loc.longitude).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_location_reader_with_mock() {
        let reader = LocationReader::new("/tmp/test.sock".to_string());

        // Without mock, should return error
        let result = reader.read_location().await;
        assert!(result.is_err());

        // With mock, should return location
        let mock_loc = Location::new(37.7749, -122.4194);
        reader.set_mock_location(Some(mock_loc.clone())).await;

        let result = reader.read_location().await;
        assert!(result.is_ok());
        let loc = result.unwrap();
        assert!((loc.latitude - 37.7749).abs() < 0.0001);
    }

    // Property 3: Location Reading During Session Start
    // Validates: Requirements 2.1, 2.2, 2.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_location_requires_both_coordinates(
            lat in -90.0f64..90.0,
            lng in -180.0f64..180.0
        ) {
            let loc = Location::new(lat, lng);

            // Both coordinates must be present for valid location
            prop_assert!(loc.is_valid());

            // Location should contain both latitude and longitude
            prop_assert!((loc.latitude - lat).abs() < 0.0001);
            prop_assert!((loc.longitude - lng).abs() < 0.0001);
        }

        #[test]
        fn prop_invalid_coordinates_rejected(
            lat in 100.0f64..1000.0,
            lng in 200.0f64..1000.0
        ) {
            let loc = Location::new(lat, lng);

            // Invalid coordinates should be rejected
            prop_assert!(!loc.is_valid());
        }
    }
}
