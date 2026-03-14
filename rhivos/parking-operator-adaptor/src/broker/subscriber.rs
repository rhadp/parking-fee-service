//! DATA_BROKER subscriber for lock/unlock events.
//!
//! [`BrokerSubscriber`] connects to the kuksa.val.v1 gRPC service and
//! subscribes to the `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` signal.
//!
//! Requirements: 08-REQ-6.1

use futures::stream::BoxStream;
use std::time::Duration;
use tracing::warn;

use crate::kuksav1;

/// VSS signal path for the driver-side door lock state.
pub const IS_LOCKED_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// Maximum connection attempts before giving up (08-REQ-6.E1).
const MAX_CONNECT_ATTEMPTS: u32 = 5;

/// Initial backoff delay in seconds for connection retries.
const INITIAL_CONNECT_BACKOFF_SECS: u64 = 1;

/// Opaque lock event emitted by the subscriber stream.
pub struct LockEvent(pub bool);

/// DATA_BROKER subscriber that streams lock/unlock events.
///
/// Holds a `tonic::transport::Channel` (internally Arc-based) established
/// during [`BrokerSubscriber::connect`].
pub struct BrokerSubscriber {
    channel: tonic::transport::Channel,
}

impl BrokerSubscriber {
    /// Connect to DATA_BROKER at `addr` with exponential-backoff retry.
    ///
    /// Makes up to 5 attempts with delays of 1 s, 2 s, 4 s, 8 s.
    /// Returns `Err` if all attempts fail (08-REQ-6.E1 — caller should exit non-zero).
    pub async fn connect(addr: &str) -> Result<Self, String> {
        let mut delay = Duration::from_secs(INITIAL_CONNECT_BACKOFF_SECS);

        for attempt in 1..=MAX_CONNECT_ATTEMPTS {
            let endpoint = tonic::transport::Endpoint::from_shared(addr.to_string())
                .map_err(|e| format!("invalid DATA_BROKER address '{}': {}", addr, e))?;

            match endpoint.connect().await {
                Ok(channel) => {
                    tracing::info!(addr, "connected to DATA_BROKER");
                    return Ok(Self { channel });
                }
                Err(e) => {
                    if attempt == MAX_CONNECT_ATTEMPTS {
                        return Err(format!(
                            "failed to connect to DATA_BROKER at '{}' after {} attempts: {}",
                            addr, attempt, e
                        ));
                    }
                    warn!(
                        attempt,
                        retry_in_secs = delay.as_secs(),
                        error = %e,
                        "DATA_BROKER connection attempt failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        }
        unreachable!()
    }

    /// Return a `BrokerSessionPublisher` that shares this subscriber's channel.
    ///
    /// This avoids a second connection when both subscriber and publisher are needed.
    pub fn make_publisher(&self) -> crate::broker::BrokerSessionPublisher {
        crate::broker::BrokerSessionPublisher::from_channel(self.channel.clone())
    }

    /// Subscribe to [`IS_LOCKED_SIGNAL`] and return a streaming event iterator.
    ///
    /// Spawns a background task that drives the server-streaming gRPC response
    /// and forwards parsed `LockEvent`s.  When the stream ends, the box stream
    /// yields `None`.
    pub async fn subscribe_lock_events(
        &mut self,
    ) -> Result<BoxStream<'static, Result<LockEvent, String>>, String> {
        use kuksav1::{val_client::ValClient, Field, SubscribeEntry, SubscribeRequest, View};

        let mut client = ValClient::new(self.channel.clone());

        let req = tonic::Request::new(SubscribeRequest {
            entries: vec![SubscribeEntry {
                path: IS_LOCKED_SIGNAL.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        });

        let grpc_stream = client
            .subscribe(req)
            .await
            .map_err(|e| format!("subscribe RPC failed: {}", e))?
            .into_inner();

        // Map the gRPC stream to our LockEvent stream.
        use futures::stream::StreamExt;
        let event_stream = grpc_stream.map(|result| match result {
            Err(e) => Err(format!("subscribe stream error: {}", e)),
            Ok(resp) => {
                // Extract the bool value from the first update.
                let locked = resp
                    .updates
                    .into_iter()
                    .next()
                    .and_then(|u| u.entry)
                    .and_then(|e| e.value)
                    .and_then(|dp| dp.value)
                    .and_then(|v| match v {
                        kuksav1::datapoint::Value::Bool(b) => Some(b),
                        _ => None,
                    });

                match locked {
                    Some(b) => Ok(LockEvent(b)),
                    None => Err("subscribe response missing bool value".to_string()),
                }
            }
        });

        Ok(Box::pin(event_stream))
    }

    /// Extract the `IsLocked` boolean from a lock event.
    pub fn extract_is_locked(event: &LockEvent) -> Option<bool> {
        Some(event.0)
    }
}
