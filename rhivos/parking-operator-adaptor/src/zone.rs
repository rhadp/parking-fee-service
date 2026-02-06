//! Zone lookup client for PARKING_FEE_SERVICE.
//!
//! This module provides zone lookup for automatic lock events.

use std::time::Duration;

use serde::Deserialize;
use tracing::debug;

use crate::error::ApiError;

/// Zone information from PARKING_FEE_SERVICE.
#[derive(Debug, Clone, Deserialize)]
pub struct ZoneInfo {
    /// Zone identifier
    pub zone_id: String,
    /// Operator name
    pub operator_name: String,
    /// Hourly rate
    pub hourly_rate: f64,
    /// Currency
    pub currency: String,
    /// Adapter image reference
    #[serde(default)]
    pub adapter_image_ref: String,
    /// Adapter checksum
    #[serde(default)]
    pub adapter_checksum: String,
}

/// HTTP client for zone lookup via PARKING_FEE_SERVICE.
pub struct ZoneLookupClient {
    http_client: reqwest::Client,
    base_url: String,
    #[allow(dead_code)]
    max_retries: u32,
    #[allow(dead_code)]
    base_delay_ms: u64,
}

impl ZoneLookupClient {
    /// Create a new ZoneLookupClient.
    pub fn new(base_url: String, max_retries: u32, base_delay_ms: u64, timeout_ms: u64) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            base_url,
            max_retries,
            base_delay_ms,
        }
    }

    /// Look up the parking zone for a location.
    ///
    /// Returns None if the location is not in a parking zone.
    pub async fn lookup_zone(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<Option<ZoneInfo>, ApiError> {
        let url = format!(
            "{}/zones/lookup?lat={}&lng={}",
            self.base_url, latitude, longitude
        );
        debug!("GET {}", url);

        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let zone_info: Option<ZoneInfo> = response.json().await.map_err(|e| {
                ApiError::InvalidResponse(format!("Failed to parse zone response: {}", e))
            })?;
            Ok(zone_info)
        } else if response.status().as_u16() == 404 {
            // No zone found at this location
            Ok(None)
        } else {
            let status = response.status().as_u16();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(ApiError::HttpError { status, message })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone_info_deserialization() {
        let json = r#"{
            "zone_id": "zone-123",
            "operator_name": "City Parking",
            "hourly_rate": 2.50,
            "currency": "USD"
        }"#;

        let zone_info: ZoneInfo = serde_json::from_str(json).unwrap();
        assert_eq!(zone_info.zone_id, "zone-123");
        assert_eq!(zone_info.operator_name, "City Parking");
        assert!((zone_info.hourly_rate - 2.50).abs() < 0.01);
        assert_eq!(zone_info.currency, "USD");
    }
}
