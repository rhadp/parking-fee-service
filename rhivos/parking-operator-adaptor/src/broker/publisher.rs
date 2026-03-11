//! DATA_BROKER publisher for session state.
//!
//! Writes `Vehicle.Parking.SessionActive` to the Kuksa Databroker
//! via gRPC (network TCP, cross-partition).

use tonic::transport::{Channel, Endpoint};
use tracing::{error, info};

use super::kuksa::val::v2::{
    val_client::ValClient, value::TypedValue, Datapoint, PublishValueRequest, SignalId, Value,
};

/// VSS signal path for the parking session active state.
pub const SESSION_ACTIVE_SIGNAL: &str = "Vehicle.Parking.SessionActive";

/// Publishes parking session state to DATA_BROKER via gRPC over TCP.
#[derive(Clone)]
pub struct BrokerPublisher {
    client: ValClient<Channel>,
}

impl BrokerPublisher {
    /// Connect to DATA_BROKER via TCP.
    ///
    /// The `addr` parameter should be a full URI like `http://localhost:55556`.
    pub async fn connect(addr: &str) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::try_from(addr.to_string())?.connect().await?;
        let client = ValClient::new(channel);
        info!("BrokerPublisher connected to DATA_BROKER at {}", addr);
        Ok(Self { client })
    }

    /// Write `Vehicle.Parking.SessionActive` to DATA_BROKER.
    ///
    /// Sets the signal to `true` when a session is active, `false` when idle.
    /// Logs errors but does not propagate them (the internal session state
    /// has already transitioned; the signal may be stale on failure).
    pub async fn set_session_active(&mut self, active: bool) -> Result<(), tonic::Status> {
        let request = PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(
                    super::kuksa::val::v2::signal_id::Signal::Path(
                        SESSION_ACTIVE_SIGNAL.to_string(),
                    ),
                ),
            }),
            data_point: Some(Datapoint {
                timestamp: None,
                value: Some(Value {
                    typed_value: Some(TypedValue::Bool(active)),
                }),
            }),
        };

        match self.client.publish_value(request).await {
            Ok(_) => {
                info!(
                    signal = SESSION_ACTIVE_SIGNAL,
                    active, "published SessionActive to DATA_BROKER"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    signal = SESSION_ACTIVE_SIGNAL,
                    active,
                    error = %e,
                    "failed to publish SessionActive to DATA_BROKER"
                );
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that SESSION_ACTIVE_SIGNAL constant matches the expected VSS path.
    #[test]
    fn test_session_active_signal_path() {
        assert_eq!(SESSION_ACTIVE_SIGNAL, "Vehicle.Parking.SessionActive");
    }

    /// Test that BrokerPublisher::connect fails gracefully when DATA_BROKER is unreachable.
    /// This validates error handling without requiring a running DATA_BROKER.
    #[tokio::test]
    async fn test_publisher_connect_fails_when_unreachable() {
        // Use an address that will not have a DATA_BROKER running
        let result = BrokerPublisher::connect("http://127.0.0.1:19999").await;
        assert!(
            result.is_err(),
            "connect should fail when DATA_BROKER is unreachable"
        );
    }

    /// Test that BrokerSubscriber::try_connect fails gracefully when DATA_BROKER is unreachable.
    #[tokio::test]
    async fn test_subscriber_connect_fails_when_unreachable() {
        let result =
            super::super::subscriber::BrokerSubscriber::try_connect("http://127.0.0.1:19999")
                .await;
        assert!(
            result.is_err(),
            "try_connect should fail when DATA_BROKER is unreachable"
        );
    }
}
