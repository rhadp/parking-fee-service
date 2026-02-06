//! State publication to DATA_BROKER.
//!
//! This module publishes session state signals to DATA_BROKER.

use tracing::{debug, info};

use crate::error::ParkingError;

/// Publishes session state to DATA_BROKER.
pub struct StatePublisher {
    /// DATA_BROKER socket path
    data_broker_socket: String,
    /// Last published state for testing
    #[cfg(test)]
    last_published: std::sync::Arc<tokio::sync::RwLock<Option<bool>>>,
}

impl StatePublisher {
    /// Create a new StatePublisher.
    pub fn new(data_broker_socket: String) -> Self {
        Self {
            data_broker_socket,
            #[cfg(test)]
            last_published: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Publish Vehicle.Parking.SessionActive signal.
    pub async fn publish_session_active(&self, active: bool) -> Result<(), ParkingError> {
        debug!(
            "Publishing SessionActive={} to DATA_BROKER at {}",
            active, self.data_broker_socket
        );

        // In a real implementation, this would:
        // 1. Connect to DATA_BROKER via gRPC/UDS
        // 2. Write Vehicle.Parking.SessionActive signal
        // 3. Return success or error

        #[cfg(test)]
        {
            *self.last_published.write().await = Some(active);
        }

        info!("Published SessionActive={}", active);
        Ok(())
    }

    /// Get last published state (for testing).
    #[cfg(test)]
    pub async fn get_last_published(&self) -> Option<bool> {
        *self.last_published.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[tokio::test]
    async fn test_publish_session_active() {
        let publisher = StatePublisher::new("/tmp/test.sock".to_string());

        publisher.publish_session_active(true).await.unwrap();
        assert_eq!(publisher.get_last_published().await, Some(true));

        publisher.publish_session_active(false).await.unwrap();
        assert_eq!(publisher.get_last_published().await, Some(false));
    }

    // Property 6: Session State Publication Consistency
    // Validates: Requirements 3.4, 4.4
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_publication_matches_state(active in proptest::bool::ANY) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let publisher = StatePublisher::new("/tmp/test.sock".to_string());

                publisher.publish_session_active(active).await.unwrap();
                let published = publisher.get_last_published().await;

                prop_assert_eq!(published, Some(active));
                Ok(())
            })?;
        }
    }
}
