//! Status poller for periodic session updates.
//!
//! This module provides periodic status polling from PARKING_OPERATOR
//! to keep session cost up to date.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::manager::SessionManager;
use crate::operator::OperatorApiClient;

/// Status poller for periodic updates.
pub struct StatusPoller {
    /// Session manager
    session_manager: Arc<SessionManager>,
    /// Operator API client
    operator_client: OperatorApiClient,
    /// Poll interval
    poll_interval: Duration,
    /// Shutdown receiver
    shutdown_rx: watch::Receiver<bool>,
}

impl StatusPoller {
    /// Create a new StatusPoller.
    pub fn new(
        session_manager: Arc<SessionManager>,
        operator_client: OperatorApiClient,
        poll_interval_ms: u64,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            session_manager,
            operator_client,
            poll_interval: Duration::from_millis(poll_interval_ms),
            shutdown_rx,
        }
    }

    /// Start the polling loop.
    pub async fn run(&mut self) {
        info!(
            "Starting status poller with interval {:?}",
            self.poll_interval
        );

        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.poll_interval) => {
                    self.poll_status().await;
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("Status poller shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Poll status for the current session.
    async fn poll_status(&self) {
        // Only poll if there's an active session
        let session = match self.session_manager.get_session().await {
            Some(s) if s.is_active() => s,
            _ => {
                debug!("No active session, skipping poll");
                return;
            }
        };

        debug!("Polling status for session {}", session.session_id);

        match self.operator_client.get_status(&session.session_id).await {
            Ok(status) => {
                debug!(
                    "Status poll: cost={}, duration={}s",
                    status.current_cost, status.duration_seconds
                );

                // Update session cost
                if let Err(e) = self.session_manager.update_cost(status.current_cost).await {
                    warn!("Failed to update session cost: {}", e);
                }
            }
            Err(e) => {
                warn!("Status poll failed: {}", e);
                // Don't fail on poll errors, just log and continue
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceConfig;
    use crate::location::LocationReader;
    use crate::publisher::StatePublisher;
    use crate::store::SessionStore;
    use crate::zone::ZoneLookupClient;
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn create_test_poller(temp_dir: &TempDir, poll_interval_ms: u64) -> (StatusPoller, watch::Sender<bool>) {
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

        let session_manager = Arc::new(SessionManager::new(
            location_reader,
            zone_lookup_client,
            operator_client.clone(),
            state_publisher,
            session_store,
        ));

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let poller = StatusPoller::new(
            session_manager,
            operator_client,
            poll_interval_ms,
            shutdown_rx,
        );

        (poller, shutdown_tx)
    }

    #[tokio::test]
    async fn test_poller_shutdown() {
        let temp_dir = TempDir::new().unwrap();
        let (mut poller, shutdown_tx) = create_test_poller(&temp_dir, 100);

        // Start poller in background
        let handle = tokio::spawn(async move {
            poller.run().await;
        });

        // Signal shutdown
        shutdown_tx.send(true).unwrap();

        // Wait for completion
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("Poller should shutdown")
            .expect("Poller should complete");
    }

    // Property 14: Poll Interval Bounds
    // Validates: Requirements 5.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_poll_interval_positive(
            interval_ms in 100u64..60000
        ) {
            let temp_dir = TempDir::new().unwrap();
            let (poller, _shutdown_tx) = create_test_poller(&temp_dir, interval_ms);

            prop_assert!(poller.poll_interval.as_millis() >= 100);
            prop_assert!(poller.poll_interval.as_millis() <= 60000);
        }
    }
}
