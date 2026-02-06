//! Session manager for parking operations.
//!
//! This module provides the SessionManager which coordinates all parking
//! session operations including start, stop, and status updates.

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::ParkingError;
use crate::location::{Location, LocationReader};
use crate::operator::OperatorApiClient;
use crate::publisher::StatePublisher;
use crate::session::{Session, SessionState};
use crate::store::SessionStore;
use crate::zone::ZoneLookupClient;

/// Manages parking session lifecycle.
pub struct SessionManager {
    /// Location reader for GPS data
    location_reader: LocationReader,
    /// Zone lookup client
    zone_lookup_client: ZoneLookupClient,
    /// Operator API client
    operator_client: OperatorApiClient,
    /// State publisher for DATA_BROKER
    state_publisher: StatePublisher,
    /// Session storage
    session_store: SessionStore,
    /// Current session
    current_session: RwLock<Option<Session>>,
}

impl SessionManager {
    /// Create a new SessionManager.
    pub fn new(
        location_reader: LocationReader,
        zone_lookup_client: ZoneLookupClient,
        operator_client: OperatorApiClient,
        state_publisher: StatePublisher,
        session_store: SessionStore,
    ) -> Self {
        Self {
            location_reader,
            zone_lookup_client,
            operator_client,
            state_publisher,
            session_store,
            current_session: RwLock::new(None),
        }
    }

    /// Initialize the session manager, recovering any persisted session.
    pub async fn init(&self) -> Result<(), ParkingError> {
        info!("Initializing SessionManager");

        // Try to recover persisted session
        if let Some(session) = self.session_store.load().await? {
            info!(
                "Recovered session {} in state {:?}",
                session.session_id, session.state
            );
            *self.current_session.write().await = Some(session);
        }

        Ok(())
    }

    /// Start a parking session automatically based on lock event.
    ///
    /// This is called when a lock event is detected. It:
    /// 1. Gets current GPS location
    /// 2. Looks up the parking zone
    /// 3. Starts a session with the parking operator
    pub async fn start_session_auto(&self) -> Result<Session, ParkingError> {
        info!("Starting automatic parking session");

        // Check if session already active
        if let Some(session) = self.current_session.read().await.as_ref() {
            if session.is_active() || session.is_in_progress() {
                warn!("Session already active or in progress: {}", session.session_id);
                return Err(ParkingError::SessionAlreadyActive(
                    session.session_id.clone(),
                ));
            }
        }

        // Get current location
        let location = self.location_reader.read_location().await?;
        debug!("Got location: {:?}", location);

        // Look up parking zone
        let zone_info = self
            .zone_lookup_client
            .lookup_zone(location.latitude, location.longitude)
            .await?
            .ok_or(ParkingError::NoZoneFound)?;

        info!("Found zone: {} ({})", zone_info.zone_id, zone_info.operator_name);

        // Start session with operator
        self.start_session(&location, &zone_info.zone_id).await
    }

    /// Start a parking session at a specific location and zone.
    pub async fn start_session(
        &self,
        location: &Location,
        zone_id: &str,
    ) -> Result<Session, ParkingError> {
        info!("Starting parking session in zone {}", zone_id);

        // Create session in Starting state
        let mut session = Session::new_starting(location.clone(), zone_id.to_string());
        *self.current_session.write().await = Some(session.clone());

        // Persist starting state
        self.session_store.save(&session).await?;

        // Call operator API
        match self.operator_client.start_session(location, zone_id).await {
            Ok(response) => {
                info!(
                    "Session started: {} (rate: {} per hour)",
                    response.session_id, response.hourly_rate
                );

                // Update session to Active
                session.set_active(response.session_id.clone(), response.hourly_rate);
                *self.current_session.write().await = Some(session.clone());

                // Persist active state
                self.session_store.save(&session).await?;

                // Publish state to DATA_BROKER
                self.state_publisher.publish_session_active(true).await?;

                Ok(session)
            }
            Err(e) => {
                error!("Failed to start session: {}", e);

                // Update session to Error
                session.set_error(format!("Start failed: {}", e));
                *self.current_session.write().await = Some(session.clone());
                self.session_store.save(&session).await?;

                Err(ParkingError::from(e))
            }
        }
    }

    /// Stop the current parking session.
    pub async fn stop_session(&self) -> Result<Session, ParkingError> {
        info!("Stopping parking session");

        let mut session = self
            .current_session
            .write()
            .await
            .take()
            .ok_or(ParkingError::NoActiveSession)?;

        if !session.is_active() {
            warn!("Session not active, state: {:?}", session.state);
            *self.current_session.write().await = Some(session.clone());
            return Err(ParkingError::NoActiveSession);
        }

        // Transition to Stopping
        session.set_stopping();
        *self.current_session.write().await = Some(session.clone());
        self.session_store.save(&session).await?;

        // Call operator API
        match self.operator_client.stop_session(&session.session_id).await {
            Ok(response) => {
                info!(
                    "Session stopped: {} (total: {} {})",
                    response.session_id, response.total_cost, "USD"
                );

                // Update session to Stopped
                session.set_stopped(response.total_cost);
                *self.current_session.write().await = Some(session.clone());

                // Persist stopped state
                self.session_store.save(&session).await?;

                // Publish state to DATA_BROKER
                self.state_publisher.publish_session_active(false).await?;

                // Clear persisted session after successful stop
                self.session_store.clear().await?;

                Ok(session)
            }
            Err(e) => {
                error!("Failed to stop session: {}", e);

                // Update session to Error but keep it active
                session.set_error(format!("Stop failed: {}", e));
                session.state = SessionState::Active; // Revert to active for retry
                *self.current_session.write().await = Some(session.clone());
                self.session_store.save(&session).await?;

                Err(ParkingError::from(e))
            }
        }
    }

    /// Get current session status.
    pub async fn get_session(&self) -> Option<Session> {
        self.current_session.read().await.clone()
    }

    /// Get current session state.
    pub async fn get_state(&self) -> SessionState {
        self.current_session
            .read()
            .await
            .as_ref()
            .map(|s| s.state)
            .unwrap_or(SessionState::None)
    }

    /// Update session cost from status poll.
    pub async fn update_cost(&self, cost: f64) -> Result<(), ParkingError> {
        let mut session_lock = self.current_session.write().await;
        if let Some(ref mut session) = *session_lock {
            if session.is_active() {
                session.update_cost(cost);
                self.session_store.save(session).await?;
                debug!("Updated session cost to {}", cost);
            }
        }
        Ok(())
    }

    /// Check if a session is currently active.
    pub async fn has_active_session(&self) -> bool {
        self.current_session
            .read()
            .await
            .as_ref()
            .map(|s| s.is_active())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceConfig;
    use proptest::prelude::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_manager(temp_dir: &TempDir) -> SessionManager {
        let config = ServiceConfig::default();
        let location_reader = LocationReader::new(config.data_broker_socket.clone());
        let zone_lookup_client = ZoneLookupClient::new(
            config.parking_fee_service_url.clone(),
            config.api_max_retries,
            config.api_base_delay_ms,
            config.api_timeout_ms,
        );
        let operator_client = OperatorApiClient::new(
            config.operator_base_url.clone(),
            config.vehicle_id.clone(),
            config.api_max_retries,
            config.api_base_delay_ms,
            config.api_max_delay_ms,
            config.api_timeout_ms,
        );
        let state_publisher = StatePublisher::new(config.data_broker_socket.clone());
        let session_store = SessionStore::new(temp_dir.path().join("session.json"));

        SessionManager::new(
            location_reader,
            zone_lookup_client,
            operator_client,
            state_publisher,
            session_store,
        )
    }

    #[tokio::test]
    async fn test_initial_state() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_test_manager(&temp_dir);

        assert!(manager.get_session().await.is_none());
        assert_eq!(manager.get_state().await, SessionState::None);
        assert!(!manager.has_active_session().await);
    }

    #[tokio::test]
    async fn test_no_active_session_error() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_test_manager(&temp_dir);

        let result = manager.stop_session().await;
        assert!(matches!(result, Err(ParkingError::NoActiveSession)));
    }

    // Property 7: Session Lifecycle State Machine
    // Validates: Requirements 3.1, 4.1, 7.1
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_session_starts_in_starting_state(
            lat in -90.0f64..90.0,
            lng in -180.0f64..180.0,
            zone_id in "[a-z0-9-]{4,20}"
        ) {
            let location = Location::new(lat, lng);
            let session = Session::new_starting(location, zone_id);

            prop_assert_eq!(session.state, SessionState::Starting);
            prop_assert!(session.session_id.is_empty());
        }

        #[test]
        fn prop_active_session_has_valid_id(
            session_id in "[a-z0-9-]{8,36}",
            hourly_rate in 0.5f64..50.0
        ) {
            let location = Location::new(37.7749, -122.4194);
            let mut session = Session::new_starting(location, "zone-1".to_string());

            session.set_active(session_id.clone(), hourly_rate);

            prop_assert_eq!(session.state, SessionState::Active);
            prop_assert_eq!(session.session_id, session_id);
            prop_assert!((session.hourly_rate - hourly_rate).abs() < 0.01);
        }
    }

    // Property 8: Cost Accumulation Correctness
    // Validates: Requirements 5.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_cost_update_preserves_value(
            initial_cost in 0.0f64..100.0,
            new_cost in 0.0f64..1000.0
        ) {
            let location = Location::new(37.7749, -122.4194);
            let mut session = Session::new_starting(location, "zone-1".to_string());
            session.set_active("session-123".to_string(), 2.50);
            session.current_cost = initial_cost;

            session.update_cost(new_cost);

            prop_assert!((session.current_cost - new_cost).abs() < 0.0001);
        }
    }

    // Property 9: Concurrent State Access Safety
    // Validates: Requirements 7.1, 7.2
    #[tokio::test]
    async fn test_concurrent_state_access() {
        let temp_dir = TempDir::new().unwrap();
        let manager = Arc::new(create_test_manager(&temp_dir));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let manager_clone = manager.clone();
                tokio::spawn(async move {
                    let _ = manager_clone.get_session().await;
                    let _ = manager_clone.get_state().await;
                    let _ = manager_clone.has_active_session().await;
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }
    }
}
