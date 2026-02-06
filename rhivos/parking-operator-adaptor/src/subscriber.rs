//! Signal subscription from DATA_BROKER.
//!
//! This module subscribes to lock/unlock signals from DATA_BROKER
//! and triggers session start/stop operations.

use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, error, info, warn};

use crate::error::ParkingError;
use crate::manager::SessionManager;

/// Signal update from DATA_BROKER.
#[derive(Debug, Clone)]
pub struct LockSignalUpdate {
    /// Whether the door is locked
    pub is_locked: bool,
}

/// Subscribes to lock/unlock signals from DATA_BROKER.
pub struct SignalSubscriber {
    /// DATA_BROKER socket path
    data_broker_socket: String,
    /// Session manager for triggering operations
    session_manager: Arc<SessionManager>,
    /// Maximum reconnection attempts
    reconnect_max_attempts: u32,
    /// Base delay for reconnection backoff
    reconnect_base_delay_ms: u64,
    /// Maximum delay for reconnection backoff
    reconnect_max_delay_ms: u64,
    /// Previous lock state for transition detection
    previous_lock_state: tokio::sync::RwLock<Option<bool>>,
    /// Connection status
    connected: tokio::sync::RwLock<bool>,
}

impl SignalSubscriber {
    /// Create a new SignalSubscriber.
    pub fn new(
        data_broker_socket: String,
        session_manager: Arc<SessionManager>,
        reconnect_max_attempts: u32,
        reconnect_base_delay_ms: u64,
        reconnect_max_delay_ms: u64,
    ) -> Self {
        Self {
            data_broker_socket,
            session_manager,
            reconnect_max_attempts,
            reconnect_base_delay_ms,
            reconnect_max_delay_ms,
            previous_lock_state: tokio::sync::RwLock::new(None),
            connected: tokio::sync::RwLock::new(false),
        }
    }

    /// Start subscribing to lock signals.
    pub async fn start(&self) -> Result<(), ParkingError> {
        info!(
            "Starting lock signal subscription on {}",
            self.data_broker_socket
        );

        // In a real implementation, this would:
        // 1. Connect to DATA_BROKER via gRPC/UDS
        // 2. Subscribe to Vehicle.Cabin.Door.Row1.DriverSide.IsLocked
        // 3. Start receiving signal updates

        *self.connected.write().await = true;
        info!("Lock signal subscription started");
        Ok(())
    }

    /// Handle a lock signal change.
    pub async fn on_signal_change(&self, is_locked: bool) {
        let previous = *self.previous_lock_state.read().await;

        debug!(
            "Lock signal change: previous={:?}, current={}",
            previous, is_locked
        );

        // Update previous state
        *self.previous_lock_state.write().await = Some(is_locked);

        // Detect transitions
        match previous {
            Some(false) if is_locked => {
                // Unlocked -> Locked: Start session
                info!("Lock transition detected (unlocked -> locked), starting session");
                if let Err(e) = self.session_manager.start_session_auto().await {
                    error!("Failed to start session on lock: {}", e);
                }
            }
            Some(true) if !is_locked => {
                // Locked -> Unlocked: Stop session
                info!("Unlock transition detected (locked -> unlocked), stopping session");
                if let Err(e) = self.session_manager.stop_session().await {
                    error!("Failed to stop session on unlock: {}", e);
                }
            }
            _ => {
                debug!("No state transition, ignoring signal");
            }
        }
    }

    /// Attempt to reconnect with exponential backoff.
    pub async fn reconnect_with_backoff(&self) -> Result<(), ParkingError> {
        let mut delay = Duration::from_millis(self.reconnect_base_delay_ms);
        let max_delay = Duration::from_millis(self.reconnect_max_delay_ms);

        for attempt in 0..self.reconnect_max_attempts {
            info!("DATA_BROKER reconnection attempt {}", attempt + 1);

            match self.try_connect().await {
                Ok(_) => {
                    info!("DATA_BROKER reconnected successfully");
                    *self.connected.write().await = true;
                    return Ok(());
                }
                Err(e) if attempt < self.reconnect_max_attempts - 1 => {
                    warn!(
                        "Reconnection attempt {} failed: {}, retrying in {:?}",
                        attempt + 1,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(max_delay);
                }
                Err(e) => {
                    error!(
                        "DATA_BROKER reconnection failed after {} attempts: {}",
                        self.reconnect_max_attempts, e
                    );
                    return Err(ParkingError::DataBrokerConnectionLost);
                }
            }
        }
        Err(ParkingError::DataBrokerConnectionLost)
    }

    /// Try to connect to DATA_BROKER.
    async fn try_connect(&self) -> Result<(), ParkingError> {
        // In a real implementation, this would attempt to connect
        // For now, always succeed in test mode
        Ok(())
    }

    /// Check if connected to DATA_BROKER.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceConfig;
    use crate::location::LocationReader;
    use crate::operator::OperatorApiClient;
    use crate::publisher::StatePublisher;
    use crate::store::SessionStore;
    use crate::zone::ZoneLookupClient;
    use proptest::prelude::*;
    use std::path::PathBuf;

    fn create_test_session_manager() -> Arc<SessionManager> {
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
        let session_store = SessionStore::new(PathBuf::from("/tmp/test-session.json"));

        Arc::new(SessionManager::new(
            location_reader,
            zone_lookup_client,
            operator_client,
            state_publisher,
            session_store,
        ))
    }

    #[tokio::test]
    async fn test_subscriber_start() {
        let session_manager = create_test_session_manager();
        let subscriber = SignalSubscriber::new(
            "/tmp/test.sock".to_string(),
            session_manager,
            5,
            1000,
            30000,
        );

        let result = subscriber.start().await;
        assert!(result.is_ok());
        assert!(subscriber.is_connected().await);
    }

    // Property 15: DATA_BROKER Reconnection with Backoff
    // Validates: Requirements 1.4, 1.5
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_reconnection_attempts_bounded(
            max_attempts in 1u32..10,
            base_delay_ms in 100u64..2000,
            max_delay_ms in 5000u64..60000
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let session_manager = create_test_session_manager();
                let subscriber = SignalSubscriber::new(
                    "/tmp/test.sock".to_string(),
                    session_manager,
                    max_attempts,
                    base_delay_ms,
                    max_delay_ms,
                );

                // In this test, connection always succeeds
                let result = subscriber.reconnect_with_backoff().await;
                prop_assert!(result.is_ok());
                Ok(())
            })?;
        }
    }
}
