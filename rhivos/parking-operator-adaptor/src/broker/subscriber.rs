//! DATA_BROKER subscription for lock/unlock events.
//!
//! Subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on the
//! Kuksa Databroker via gRPC (network TCP, cross-partition).

use std::time::Duration;

use tonic::transport::{Channel, Endpoint};
use tracing::{info, warn};

use super::kuksa::val::v2::{
    val_client::ValClient, SubscribeRequest, SubscribeResponse,
};

/// VSS signal path for the driver-side door lock state.
pub const IS_LOCKED_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// Subscribes to lock/unlock events from DATA_BROKER via gRPC over TCP.
#[derive(Clone)]
pub struct BrokerSubscriber {
    client: ValClient<Channel>,
}

impl BrokerSubscriber {
    /// Maximum backoff duration for retry attempts.
    const MAX_BACKOFF: Duration = Duration::from_secs(30);

    /// Connect to DATA_BROKER via TCP with exponential backoff retry.
    ///
    /// The `addr` parameter should be a full URI like `http://localhost:55556`.
    /// Retries with backoff (1s, 2s, 4s, ..., max 30s) until connected.
    pub async fn connect(addr: &str) -> Result<Self, tonic::transport::Error> {
        let mut backoff = Duration::from_secs(1);

        loop {
            match Self::try_connect(addr).await {
                Ok(subscriber) => {
                    info!("BrokerSubscriber connected to DATA_BROKER at {}", addr);
                    return Ok(subscriber);
                }
                Err(e) => {
                    warn!(
                        "Failed to connect to DATA_BROKER at {}: {}. Retrying in {:?}...",
                        addr, e, backoff
                    );
                    tokio::time::sleep(backoff).await;
                    backoff = std::cmp::min(backoff * 2, Self::MAX_BACKOFF);
                }
            }
        }
    }

    /// Attempt a single TCP connection to DATA_BROKER.
    pub async fn try_connect(addr: &str) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::try_from(addr.to_string())?.connect().await?;
        let client = ValClient::new(channel);
        Ok(Self { client })
    }

    /// Subscribe to lock/unlock events on `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`.
    ///
    /// Returns a streaming response of signal updates. Each update contains
    /// a map of signal path -> datapoint. The caller should extract the bool
    /// value from the `IsLocked` entry.
    pub async fn subscribe_lock_events(
        &mut self,
    ) -> Result<tonic::Streaming<SubscribeResponse>, tonic::Status> {
        let request = SubscribeRequest {
            signal_paths: vec![IS_LOCKED_SIGNAL.to_string()],
            buffer_size: 0,
        };

        let response = self.client.subscribe(request).await?;
        Ok(response.into_inner())
    }

    /// Extract the `IsLocked` boolean value from a SubscribeResponse.
    ///
    /// Returns `Some(true)` for locked, `Some(false)` for unlocked,
    /// or `None` if the signal is not present or has no boolean value.
    pub fn extract_is_locked(response: &SubscribeResponse) -> Option<bool> {
        response
            .entries
            .get(IS_LOCKED_SIGNAL)
            .and_then(|dp| dp.value.as_ref())
            .and_then(|v| v.typed_value.as_ref())
            .and_then(|tv| {
                if let super::kuksa::val::v2::value::TypedValue::Bool(b) = tv {
                    Some(*b)
                } else {
                    None
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::kuksa::val::v2::{Datapoint, Value, value::TypedValue};
    use std::collections::HashMap;

    /// Test that extract_is_locked returns Some(true) for a locked signal.
    #[test]
    fn test_extract_is_locked_true() {
        let mut entries = HashMap::new();
        entries.insert(
            IS_LOCKED_SIGNAL.to_string(),
            Datapoint {
                timestamp: None,
                value: Some(Value {
                    typed_value: Some(TypedValue::Bool(true)),
                }),
            },
        );
        let response = SubscribeResponse { entries };
        assert_eq!(BrokerSubscriber::extract_is_locked(&response), Some(true));
    }

    /// Test that extract_is_locked returns Some(false) for an unlocked signal.
    #[test]
    fn test_extract_is_locked_false() {
        let mut entries = HashMap::new();
        entries.insert(
            IS_LOCKED_SIGNAL.to_string(),
            Datapoint {
                timestamp: None,
                value: Some(Value {
                    typed_value: Some(TypedValue::Bool(false)),
                }),
            },
        );
        let response = SubscribeResponse { entries };
        assert_eq!(BrokerSubscriber::extract_is_locked(&response), Some(false));
    }

    /// Test that extract_is_locked returns None when signal is missing.
    #[test]
    fn test_extract_is_locked_missing_signal() {
        let response = SubscribeResponse {
            entries: HashMap::new(),
        };
        assert_eq!(BrokerSubscriber::extract_is_locked(&response), None);
    }

    /// Test that extract_is_locked returns None for non-bool value.
    #[test]
    fn test_extract_is_locked_wrong_type() {
        let mut entries = HashMap::new();
        entries.insert(
            IS_LOCKED_SIGNAL.to_string(),
            Datapoint {
                timestamp: None,
                value: Some(Value {
                    typed_value: Some(TypedValue::String("not_a_bool".to_string())),
                }),
            },
        );
        let response = SubscribeResponse { entries };
        assert_eq!(BrokerSubscriber::extract_is_locked(&response), None);
    }

    /// Test that IS_LOCKED_SIGNAL constant matches the expected VSS path.
    #[test]
    fn test_is_locked_signal_path() {
        assert_eq!(
            IS_LOCKED_SIGNAL,
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
        );
    }
}
