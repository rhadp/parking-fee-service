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

/// Retry configuration for testing.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Base delay in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
        }
    }
}

/// Result of a retry operation with tracking info.
#[derive(Debug, Clone)]
pub struct RetryResult<T> {
    /// The result of the operation
    pub result: Result<T, ApiError>,
    /// Number of attempts made
    pub attempts: u32,
    /// Delays between attempts (in milliseconds)
    pub delays: Vec<u64>,
}

/// Execute an operation with retry and track the behavior.
/// This is exposed for testing purposes.
pub async fn call_with_retry_tracked<T, F, Fut>(
    config: &RetryConfig,
    mut f: F,
) -> RetryResult<T>
where
    F: FnMut(u32) -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    let mut delay_ms = config.base_delay_ms;
    let mut attempts = 0;
    let mut delays = Vec::new();

    loop {
        attempts += 1;
        match f(attempts).await {
            Ok(result) => {
                return RetryResult {
                    result: Ok(result),
                    attempts,
                    delays,
                };
            }
            Err(e) if e.is_retryable() && attempts < config.max_retries => {
                delays.push(delay_ms);
                // In tests, we don't actually sleep - just record the delay
                #[cfg(not(test))]
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(config.max_delay_ms);
            }
            Err(e) => {
                return RetryResult {
                    result: Err(e),
                    attempts,
                    delays,
                };
            }
        }
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

    // Property 7: API Retry with Exponential Backoff
    // Validates: Requirements 3.5, 4.5
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_retry_with_exponential_backoff(
            max_retries in 2u32..6,
            base_delay_ms in 100u64..2000,
            max_delay_ms in 5000u64..60000,
            fail_count in 1u32..5
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = RetryConfig {
                    max_retries,
                    base_delay_ms,
                    max_delay_ms,
                };

                // Determine how many times we'll actually fail before success/giving up
                let actual_fails = fail_count.min(max_retries);
                let will_succeed = fail_count < max_retries;

                let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
                let call_count_clone = call_count.clone();

                let result = call_with_retry_tracked(&config, |attempt| {
                    let cc = call_count_clone.clone();
                    async move {
                        cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        if attempt <= actual_fails {
                            // Return retryable error
                            Err(ApiError::NetworkError("simulated failure".to_string()))
                        } else {
                            Ok("success".to_string())
                        }
                    }
                }).await;

                // Verify retry count
                if will_succeed {
                    prop_assert!(result.result.is_ok());
                    prop_assert_eq!(result.attempts, actual_fails + 1);
                } else {
                    prop_assert!(result.result.is_err());
                    prop_assert_eq!(result.attempts, max_retries);
                }

                // Verify exponential backoff delays
                if result.delays.len() > 1 {
                    for i in 1..result.delays.len() {
                        let expected_delay = (result.delays[i - 1] * 2).min(max_delay_ms);
                        prop_assert_eq!(result.delays[i], expected_delay,
                            "Delay at index {} should be double the previous (or capped at max)", i);
                    }
                }

                // Verify delays are capped at max_delay_ms
                for delay in &result.delays {
                    prop_assert!(*delay <= max_delay_ms,
                        "Delay {} should not exceed max_delay_ms {}", delay, max_delay_ms);
                }

                // Verify first delay equals base_delay_ms
                if !result.delays.is_empty() {
                    prop_assert_eq!(result.delays[0], base_delay_ms,
                        "First delay should equal base_delay_ms");
                }

                Ok(())
            })?;
        }
    }

    // Property 8: Error State After Retry Exhaustion
    // Validates: Requirements 3.6, 4.6
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_error_state_after_retry_exhaustion(
            max_retries in 1u32..5,
            error_message in "[a-zA-Z0-9 ]{5,50}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = RetryConfig {
                    max_retries,
                    base_delay_ms: 100,
                    max_delay_ms: 1000,
                };

                // Always fail with a retryable error
                let result: RetryResult<String> = call_with_retry_tracked(&config, |_attempt| {
                    let msg = error_message.clone();
                    async move {
                        Err(ApiError::NetworkError(msg))
                    }
                }).await;

                // After all retries exhausted, result should be error
                prop_assert!(result.result.is_err(), "Result should be error after all retries");
                prop_assert_eq!(result.attempts, max_retries,
                    "Should have made exactly max_retries attempts");

                // Error message should be preserved
                if let Err(ApiError::NetworkError(msg)) = &result.result {
                    prop_assert_eq!(msg, &error_message, "Error message should be preserved");
                } else {
                    prop_assert!(false, "Should be NetworkError");
                }

                // Verify we recorded the right number of delays (one less than attempts)
                prop_assert_eq!(result.delays.len() as u32, max_retries - 1,
                    "Should have max_retries - 1 delays between attempts");

                Ok(())
            })?;
        }

        #[test]
        fn prop_non_retryable_error_fails_immediately(
            max_retries in 2u32..5,
            status_code in 400u16..500
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = RetryConfig {
                    max_retries,
                    base_delay_ms: 100,
                    max_delay_ms: 1000,
                };

                // Return a non-retryable HTTP error (4xx)
                let result: RetryResult<String> = call_with_retry_tracked(&config, |_attempt| {
                    async move {
                        Err(ApiError::HttpError {
                            status: status_code,
                            message: "Client error".to_string(),
                        })
                    }
                }).await;

                // Non-retryable errors should fail immediately without retries
                prop_assert!(result.result.is_err(), "Result should be error");
                prop_assert_eq!(result.attempts, 1, "Should fail after first attempt for non-retryable error");
                prop_assert!(result.delays.is_empty(), "Should have no delays for immediate failure");

                Ok(())
            })?;
        }
    }

    #[test]
    fn test_api_error_is_retryable() {
        // Network errors are retryable
        assert!(ApiError::NetworkError("connection refused".to_string()).is_retryable());

        // Timeouts are retryable
        assert!(ApiError::Timeout(10000).is_retryable());

        // 5xx errors are retryable
        assert!(ApiError::HttpError {
            status: 500,
            message: "Internal Server Error".to_string()
        }.is_retryable());
        assert!(ApiError::HttpError {
            status: 503,
            message: "Service Unavailable".to_string()
        }.is_retryable());

        // 4xx errors are NOT retryable
        assert!(!ApiError::HttpError {
            status: 400,
            message: "Bad Request".to_string()
        }.is_retryable());
        assert!(!ApiError::HttpError {
            status: 404,
            message: "Not Found".to_string()
        }.is_retryable());

        // Invalid response is NOT retryable
        assert!(!ApiError::InvalidResponse("bad json".to_string()).is_retryable());
    }
}
