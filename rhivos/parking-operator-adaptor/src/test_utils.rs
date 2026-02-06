//! Test utilities and mock services.
//!
//! This module provides mock implementations for testing.

use tokio::sync::RwLock;

use crate::error::{ApiError, ParkingError};
use crate::location::Location;
use crate::operator::{StartResponse, StatusResponse, StopResponse};
use crate::session::{Session, SessionState};
use crate::zone::ZoneInfo;

/// Mock operator API for testing.
pub struct MockOperatorApi {
    /// Start session result
    start_result: RwLock<Option<Result<StartResponse, ApiError>>>,
    /// Stop session result
    stop_result: RwLock<Option<Result<StopResponse, ApiError>>>,
    /// Status result
    status_result: RwLock<Option<Result<StatusResponse, ApiError>>>,
    /// Call history
    calls: RwLock<Vec<String>>,
}

impl MockOperatorApi {
    /// Create a new MockOperatorApi.
    pub fn new() -> Self {
        Self {
            start_result: RwLock::new(None),
            stop_result: RwLock::new(None),
            status_result: RwLock::new(None),
            calls: RwLock::new(Vec::new()),
        }
    }

    /// Set the start session result.
    pub async fn set_start_result(&self, result: Result<StartResponse, ApiError>) {
        *self.start_result.write().await = Some(result);
    }

    /// Set the stop session result.
    pub async fn set_stop_result(&self, result: Result<StopResponse, ApiError>) {
        *self.stop_result.write().await = Some(result);
    }

    /// Set the status result.
    pub async fn set_status_result(&self, result: Result<StatusResponse, ApiError>) {
        *self.status_result.write().await = Some(result);
    }

    /// Get call history.
    pub async fn get_calls(&self) -> Vec<String> {
        self.calls.read().await.clone()
    }

    /// Clear call history.
    pub async fn clear_calls(&self) {
        self.calls.write().await.clear();
    }

    /// Mock start session.
    pub async fn start_session(
        &self,
        _location: &Location,
        zone_id: &str,
    ) -> Result<StartResponse, ApiError> {
        self.calls
            .write()
            .await
            .push(format!("start_session:{}", zone_id));

        self.start_result
            .read()
            .await
            .clone()
            .unwrap_or_else(|| {
                Ok(StartResponse {
                    session_id: "mock-session-123".to_string(),
                    zone_id: zone_id.to_string(),
                    hourly_rate: 2.50,
                    start_time: "2024-01-01T00:00:00Z".to_string(),
                })
            })
    }

    /// Mock stop session.
    pub async fn stop_session(&self, session_id: &str) -> Result<StopResponse, ApiError> {
        self.calls
            .write()
            .await
            .push(format!("stop_session:{}", session_id));

        self.stop_result.read().await.clone().unwrap_or_else(|| {
            Ok(StopResponse {
                session_id: session_id.to_string(),
                start_time: "2024-01-01T00:00:00Z".to_string(),
                end_time: "2024-01-01T01:00:00Z".to_string(),
                duration_seconds: 3600,
                total_cost: 2.50,
                payment_status: "paid".to_string(),
            })
        })
    }

    /// Mock get status.
    pub async fn get_status(&self, session_id: &str) -> Result<StatusResponse, ApiError> {
        self.calls
            .write()
            .await
            .push(format!("get_status:{}", session_id));

        self.status_result.read().await.clone().unwrap_or_else(|| {
            Ok(StatusResponse {
                session_id: session_id.to_string(),
                state: "active".to_string(),
                start_time: "2024-01-01T00:00:00Z".to_string(),
                duration_seconds: 1800,
                current_cost: 1.25,
                zone_id: "zone-1".to_string(),
            })
        })
    }
}

impl Default for MockOperatorApi {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock zone lookup for testing.
pub struct MockZoneLookup {
    /// Zone info result
    zone_result: RwLock<Option<Result<Option<ZoneInfo>, ApiError>>>,
    /// Call history
    calls: RwLock<Vec<(f64, f64)>>,
}

impl MockZoneLookup {
    /// Create a new MockZoneLookup.
    pub fn new() -> Self {
        Self {
            zone_result: RwLock::new(None),
            calls: RwLock::new(Vec::new()),
        }
    }

    /// Set the zone lookup result.
    pub async fn set_zone_result(&self, result: Result<Option<ZoneInfo>, ApiError>) {
        *self.zone_result.write().await = Some(result);
    }

    /// Get call history.
    pub async fn get_calls(&self) -> Vec<(f64, f64)> {
        self.calls.read().await.clone()
    }

    /// Mock lookup zone.
    pub async fn lookup_zone(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<Option<ZoneInfo>, ApiError> {
        self.calls.write().await.push((latitude, longitude));

        self.zone_result.read().await.clone().unwrap_or_else(|| {
            Ok(Some(ZoneInfo {
                zone_id: "mock-zone-1".to_string(),
                operator_name: "Mock Operator".to_string(),
                hourly_rate: 2.50,
                currency: "USD".to_string(),
                adapter_image_ref: String::new(),
                adapter_checksum: String::new(),
            }))
        })
    }
}

impl Default for MockZoneLookup {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock DATA_BROKER for testing.
pub struct MockDataBroker {
    /// Published states
    published_states: RwLock<Vec<bool>>,
    /// Location to return
    location: RwLock<Location>,
    /// Lock state
    lock_state: RwLock<bool>,
}

impl MockDataBroker {
    /// Create a new MockDataBroker.
    pub fn new() -> Self {
        Self {
            published_states: RwLock::new(Vec::new()),
            location: RwLock::new(Location::new(37.7749, -122.4194)),
            lock_state: RwLock::new(false),
        }
    }

    /// Set the location to return.
    pub async fn set_location(&self, location: Location) {
        *self.location.write().await = location;
    }

    /// Set the lock state.
    pub async fn set_lock_state(&self, is_locked: bool) {
        *self.lock_state.write().await = is_locked;
    }

    /// Get the current location.
    pub async fn read_location(&self) -> Result<Location, ParkingError> {
        Ok(self.location.read().await.clone())
    }

    /// Get the current lock state.
    pub async fn read_lock_state(&self) -> Result<bool, ParkingError> {
        Ok(*self.lock_state.read().await)
    }

    /// Publish session active state.
    pub async fn publish_session_active(&self, active: bool) -> Result<(), ParkingError> {
        self.published_states.write().await.push(active);
        Ok(())
    }

    /// Get published states history.
    pub async fn get_published_states(&self) -> Vec<bool> {
        self.published_states.read().await.clone()
    }
}

impl Default for MockDataBroker {
    fn default() -> Self {
        Self::new()
    }
}

/// Test session builder for creating sessions with various states.
pub struct TestSessionBuilder {
    session: Session,
}

impl TestSessionBuilder {
    /// Create a new test session builder.
    pub fn new() -> Self {
        let location = Location::new(37.7749, -122.4194);
        Self {
            session: Session::new_starting(location, "test-zone".to_string()),
        }
    }

    /// Set location.
    pub fn with_location(mut self, lat: f64, lng: f64) -> Self {
        self.session.location = Location::new(lat, lng);
        self
    }

    /// Set zone ID.
    pub fn with_zone_id(mut self, zone_id: &str) -> Self {
        self.session.zone_id = zone_id.to_string();
        self
    }

    /// Set session ID.
    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.session.session_id = session_id.to_string();
        self
    }

    /// Set state to Active.
    pub fn active(mut self, session_id: &str, hourly_rate: f64) -> Self {
        self.session.set_active(session_id.to_string(), hourly_rate);
        self
    }

    /// Set state to Stopped.
    pub fn stopped(mut self, final_cost: f64) -> Self {
        self.session.set_stopped(final_cost);
        self
    }

    /// Set state to Error.
    pub fn error(mut self, message: &str) -> Self {
        self.session.set_error(message.to_string());
        self
    }

    /// Set current cost.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.session.current_cost = cost;
        self
    }

    /// Build the session.
    pub fn build(self) -> Session {
        self.session
    }
}

impl Default for TestSessionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[tokio::test]
    async fn test_mock_operator_api_default() {
        let mock = MockOperatorApi::new();
        let location = Location::new(37.7749, -122.4194);

        let result = mock.start_session(&location, "zone-1").await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.session_id, "mock-session-123");
    }

    #[tokio::test]
    async fn test_mock_operator_api_custom_result() {
        let mock = MockOperatorApi::new();
        mock.set_start_result(Err(ApiError::HttpError {
            status: 500,
            message: "Internal error".to_string(),
        }))
        .await;

        let location = Location::new(37.7749, -122.4194);
        let result = mock.start_session(&location, "zone-1").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_zone_lookup() {
        let mock = MockZoneLookup::new();

        let result = mock.lookup_zone(37.7749, -122.4194).await;
        assert!(result.is_ok());

        let zone = result.unwrap().unwrap();
        assert_eq!(zone.zone_id, "mock-zone-1");
    }

    #[tokio::test]
    async fn test_mock_data_broker() {
        let mock = MockDataBroker::new();

        mock.set_location(Location::new(40.7128, -74.0060)).await;
        let location = mock.read_location().await.unwrap();

        assert!((location.latitude - 40.7128).abs() < 0.0001);
        assert!((location.longitude - (-74.0060)).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_test_session_builder() {
        let session = TestSessionBuilder::new()
            .with_location(40.7128, -74.0060)
            .with_zone_id("nyc-zone-1")
            .active("session-456", 3.00)
            .with_cost(1.50)
            .build();

        assert_eq!(session.state, SessionState::Active);
        assert_eq!(session.session_id, "session-456");
        assert_eq!(session.zone_id, "nyc-zone-1");
        assert!((session.hourly_rate - 3.00).abs() < 0.01);
        assert!((session.current_cost - 1.50).abs() < 0.01);
    }

    // Property 18: Mock API Call Recording
    // Validates: Test infrastructure
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_mock_records_all_calls(
            zone_id in "[a-z0-9-]{4,20}",
            session_id in "[a-z0-9-]{8,36}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mock = MockOperatorApi::new();
                let location = Location::new(37.7749, -122.4194);

                let _ = mock.start_session(&location, &zone_id).await;
                let _ = mock.stop_session(&session_id).await;
                let _ = mock.get_status(&session_id).await;

                let calls = mock.get_calls().await;
                prop_assert_eq!(calls.len(), 3);
                prop_assert!(calls[0].starts_with("start_session:"));
                prop_assert!(calls[1].starts_with("stop_session:"));
                prop_assert!(calls[2].starts_with("get_status:"));
                Ok(())
            })?;
        }
    }

    // Property 19: Test Session Builder State Consistency
    // Validates: Test infrastructure
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_session_builder_state_valid(
            lat in -90.0f64..90.0,
            lng in -180.0f64..180.0,
            zone_id in "[a-z0-9-]{4,20}",
            session_id in "[a-z0-9-]{8,36}",
            hourly_rate in 0.5f64..50.0
        ) {
            let session = TestSessionBuilder::new()
                .with_location(lat, lng)
                .with_zone_id(&zone_id)
                .active(&session_id, hourly_rate)
                .build();

            prop_assert_eq!(session.state, SessionState::Active);
            prop_assert_eq!(session.session_id, session_id);
            prop_assert_eq!(session.zone_id, zone_id);
            prop_assert!((session.location.latitude - lat).abs() < 0.0001);
            prop_assert!((session.location.longitude - lng).abs() < 0.0001);
            prop_assert!((session.hourly_rate - hourly_rate).abs() < 0.01);
        }
    }
}
