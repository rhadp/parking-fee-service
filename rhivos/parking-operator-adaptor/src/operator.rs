//! PARKING_OPERATOR API client.
//!
//! This module provides HTTP client for communicating with the external
//! parking operator REST API.

use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::error::ApiError;
use crate::location::Location;

/// Request to start a parking session.
#[derive(Debug, Serialize)]
pub struct StartRequest {
    pub vehicle_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub zone_id: String,
    pub timestamp: String,
}

/// Response from starting a parking session.
#[derive(Debug, Deserialize, Clone)]
pub struct StartResponse {
    pub session_id: String,
    pub zone_id: String,
    pub hourly_rate: f64,
    pub start_time: String,
}

/// Request to stop a parking session.
#[derive(Debug, Serialize)]
pub struct StopRequest {
    pub session_id: String,
    pub timestamp: String,
}

/// Response from stopping a parking session.
#[derive(Debug, Deserialize, Clone)]
pub struct StopResponse {
    pub session_id: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_seconds: i64,
    pub total_cost: f64,
    pub payment_status: String,
}

/// Response from status query.
#[derive(Debug, Deserialize, Clone)]
pub struct StatusResponse {
    pub session_id: String,
    pub state: String,
    pub start_time: String,
    pub duration_seconds: i64,
    pub current_cost: f64,
    pub zone_id: String,
}

/// HTTP client for PARKING_OPERATOR REST API.
#[derive(Clone)]
pub struct OperatorApiClient {
    http_client: reqwest::Client,
    base_url: String,
    vehicle_id: String,
    max_retries: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
}

impl OperatorApiClient {
    /// Create a new OperatorApiClient.
    pub fn new(
        base_url: String,
        vehicle_id: String,
        max_retries: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
        timeout_ms: u64,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            base_url,
            vehicle_id,
            max_retries,
            base_delay_ms,
            max_delay_ms,
        }
    }

    /// Start a parking session.
    pub async fn start_session(
        &self,
        location: &Location,
        zone_id: &str,
    ) -> Result<StartResponse, ApiError> {
        let request = StartRequest {
            vehicle_id: self.vehicle_id.clone(),
            latitude: location.latitude,
            longitude: location.longitude,
            zone_id: zone_id.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };

        self.call_with_retry("start_session", || async {
            let url = format!("{}/parking/start", self.base_url);
            debug!("POST {} with zone_id={}", url, zone_id);

            let response = self
                .http_client
                .post(&url)
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let start_response: StartResponse = response.json().await.map_err(|e| {
                    ApiError::InvalidResponse(format!("Failed to parse start response: {}", e))
                })?;
                Ok(start_response)
            } else {
                let status = response.status().as_u16();
                let message = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                Err(ApiError::HttpError { status, message })
            }
        })
        .await
    }

    /// Stop a parking session.
    pub async fn stop_session(&self, session_id: &str) -> Result<StopResponse, ApiError> {
        let request = StopRequest {
            session_id: session_id.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };

        self.call_with_retry("stop_session", || async {
            let url = format!("{}/parking/stop", self.base_url);
            debug!("POST {} with session_id={}", url, session_id);

            let response = self
                .http_client
                .post(&url)
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let stop_response: StopResponse = response.json().await.map_err(|e| {
                    ApiError::InvalidResponse(format!("Failed to parse stop response: {}", e))
                })?;
                Ok(stop_response)
            } else {
                let status = response.status().as_u16();
                let message = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                Err(ApiError::HttpError { status, message })
            }
        })
        .await
    }

    /// Get session status.
    pub async fn get_status(&self, session_id: &str) -> Result<StatusResponse, ApiError> {
        let url = format!("{}/parking/status/{}", self.base_url, session_id);
        debug!("GET {}", url);

        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let status_response: StatusResponse = response.json().await.map_err(|e| {
                ApiError::InvalidResponse(format!("Failed to parse status response: {}", e))
            })?;
            Ok(status_response)
        } else {
            let status = response.status().as_u16();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(ApiError::HttpError { status, message })
        }
    }

    /// Call with retry and exponential backoff.
    async fn call_with_retry<T, F, Fut>(&self, operation: &str, f: F) -> Result<T, ApiError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, ApiError>>,
    {
        let mut delay = Duration::from_millis(self.base_delay_ms);
        let max_delay = Duration::from_millis(self.max_delay_ms);

        for attempt in 0..self.max_retries {
            match f().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("{} succeeded after {} attempts", operation, attempt + 1);
                    }
                    return Ok(result);
                }
                Err(e) if e.is_retryable() && attempt < self.max_retries - 1 => {
                    warn!(
                        "{} attempt {} failed: {}, retrying in {:?}",
                        operation,
                        attempt + 1,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(max_delay);
                }
                Err(e) => {
                    error!(
                        "{} failed after {} attempts: {}",
                        operation,
                        attempt + 1,
                        e
                    );
                    return Err(e);
                }
            }
        }
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_start_request_serialization() {
        let request = StartRequest {
            vehicle_id: "vehicle-1".to_string(),
            latitude: 37.7749,
            longitude: -122.4194,
            zone_id: "zone-1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"vehicle_id\":\"vehicle-1\""));
        assert!(json.contains("\"zone_id\":\"zone-1\""));
        assert!(json.contains("\"latitude\":37.7749"));
    }

    #[test]
    fn test_stop_request_serialization() {
        let request = StopRequest {
            session_id: "session-123".to_string(),
            timestamp: "2024-01-01T01:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"session_id\":\"session-123\""));
        assert!(json.contains("\"timestamp\""));
    }

    // Property 4: Session Start API Request Completeness
    // Validates: Requirements 3.1, 3.2, 3.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_start_request_completeness(
            vehicle_id in "[a-z0-9-]{5,20}",
            lat in -90.0f64..90.0,
            lng in -180.0f64..180.0,
            zone_id in "[a-z0-9-]{4,20}"
        ) {
            let request = StartRequest {
                vehicle_id: vehicle_id.clone(),
                latitude: lat,
                longitude: lng,
                zone_id: zone_id.clone(),
                timestamp: Utc::now().to_rfc3339(),
            };

            // Verify serialization includes all fields
            let json = serde_json::to_string(&request).unwrap();
            prop_assert!(json.contains("vehicle_id"));
            prop_assert!(json.contains("latitude"));
            prop_assert!(json.contains("longitude"));
            prop_assert!(json.contains("zone_id"));
            prop_assert!(json.contains("timestamp"));

            // Verify all required fields are present
            prop_assert_eq!(&request.vehicle_id, &vehicle_id);
            prop_assert!((request.latitude - lat).abs() < 0.0001);
            prop_assert!((request.longitude - lng).abs() < 0.0001);
            prop_assert_eq!(&request.zone_id, &zone_id);
            prop_assert!(!request.timestamp.is_empty());
        }
    }

    // Property 5: Session Stop API Request Completeness
    // Validates: Requirements 4.1, 4.2, 4.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_stop_request_completeness(
            session_id in "[a-z0-9-]{8,36}"
        ) {
            let request = StopRequest {
                session_id: session_id.clone(),
                timestamp: Utc::now().to_rfc3339(),
            };

            // Verify serialization includes all fields
            let json = serde_json::to_string(&request).unwrap();
            prop_assert!(json.contains("session_id"));
            prop_assert!(json.contains("timestamp"));

            // Verify all required fields are present
            prop_assert_eq!(&request.session_id, &session_id);
            prop_assert!(!request.timestamp.is_empty());
        }
    }
}
