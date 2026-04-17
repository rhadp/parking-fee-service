//! DATA_BROKER gRPC client (kuksa.val.v1).
//!
//! `BrokerClient` connects to Eclipse Kuksa Databroker and provides:
//! - `set_bool` — publish a boolean VSS signal (e.g. Vehicle.Parking.SessionActive)
//! - `subscribe_bool` — stream changes to a boolean VSS signal (e.g. IsLocked)

use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use tonic::transport::{Channel, Endpoint};
use tracing::{error, info, warn};

/// Generated types from `proto/kuksa/val.proto`.
///
/// `enum_variant_names` is suppressed because generated `Value` variants
/// follow the protobuf naming convention (`BoolValue`, `StringValue`, …)
/// which clippy flags but we cannot change.
#[allow(clippy::enum_variant_names)]
mod kuksa {
    tonic::include_proto!("kuksa");
}

use kuksa::val_service_client::ValServiceClient;
use kuksa::{DataEntry, Datapoint, Field, SetRequest, SubscribeRequest};

/// VSS signal paths used by PARKING_OPERATOR_ADAPTOR.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Errors from DATA_BROKER operations.
#[derive(Debug)]
pub enum BrokerError {
    ConnectionFailed(String),
    PublishFailed(String),
}

/// Abstraction over DATA_BROKER signal publishing.
///
/// Implemented by `BrokerClient` (production) and mock types in tests.
/// No `Send + Sync` bound so that `RefCell`-based mocks work in
/// single-threaded (`current_thread`) tokio tests.
#[allow(async_fn_in_trait)]
pub trait SessionPublisher {
    async fn set_session_active(&self, active: bool) -> Result<(), BrokerError>;
}

// ── Connection retry constants ────────────────────────────────────────────────

/// Maximum total attempts for connecting to DATA_BROKER (08-REQ-3.E3).
const MAX_CONNECT_ATTEMPTS: u32 = 5;

/// Base backoff delay in milliseconds; doubles each retry.
const BASE_BACKOFF_MS: u64 = 1_000;

// ── Production client ─────────────────────────────────────────────────────────

/// gRPC client for DATA_BROKER (kuksa.val.v1).
///
/// The underlying `Channel` is shared/multiplexed, so cloning the
/// `ValServiceClient` wrapper per call is cheap and safe.
pub struct BrokerClient {
    channel: Channel,
}

impl BrokerClient {
    /// Connect to DATA_BROKER at `addr` with exponential-backoff retry.
    ///
    /// Attempts: 1 initial + up to 4 retries with delays 1s, 2s, 4s, 8s.
    /// Returns `Err` after exhausting all attempts (08-REQ-3.E3).
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let endpoint = Endpoint::from_shared(addr.to_owned())
            .map_err(|e| BrokerError::ConnectionFailed(format!("Invalid address {addr}: {e}")))?
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10));

        let mut last_err = String::new();

        for attempt in 0..MAX_CONNECT_ATTEMPTS {
            if attempt > 0 {
                let delay_ms = BASE_BACKOFF_MS * (1u64 << (attempt - 1));
                warn!(attempt, delay_ms, "Retrying DATA_BROKER connection");
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }

            match endpoint.connect().await {
                Ok(channel) => {
                    info!(addr, "Connected to DATA_BROKER");
                    return Ok(BrokerClient { channel });
                }
                Err(e) => {
                    last_err = e.to_string();
                    warn!(attempt, error = %e, "Failed to connect to DATA_BROKER");
                }
            }
        }

        Err(BrokerError::ConnectionFailed(format!(
            "Failed to connect to DATA_BROKER at {addr} after {MAX_CONNECT_ATTEMPTS} attempts: {last_err}"
        )))
    }

    /// Publish a boolean value to a VSS signal via kuksa Set RPC.
    pub async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let mut client = ValServiceClient::new(self.channel.clone());
        let response = client
            .set(SetRequest {
                updates: vec![DataEntry {
                    path: signal.to_owned(),
                    value: Some(Datapoint {
                        timestamp: 0,
                        value: Some(kuksa::datapoint::Value::BoolValue(value)),
                    }),
                }],
            })
            .await
            .map_err(|e| BrokerError::PublishFailed(format!("SET {signal}={value} failed: {e}")))?
            .into_inner();

        if !response.errors.is_empty() {
            let msg = response
                .errors
                .iter()
                .map(|e| format!("[{}] {}: {}", e.code, e.reason, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(BrokerError::PublishFailed(format!(
                "SET {signal}={value} returned field errors: {msg}"
            )));
        }

        Ok(())
    }

    /// Subscribe to a boolean VSS signal; returns a channel of bool values.
    ///
    /// Spawns a background task that forwards each `BoolValue` update from
    /// the DATA_BROKER stream to the returned receiver.  The channel closes
    /// when the stream ends or the receiver is dropped.
    pub async fn subscribe_bool(
        &self,
        signal: &str,
    ) -> Result<mpsc::Receiver<bool>, BrokerError> {
        let mut client = ValServiceClient::new(self.channel.clone());
        let request = SubscribeRequest {
            entries: vec![Field {
                path: signal.to_owned(),
            }],
        };

        let stream = client
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::ConnectionFailed(format!("Subscribe to {signal} failed: {e}")))?
            .into_inner();

        let (tx, rx) = mpsc::channel::<bool>(128);

        tokio::spawn(async move {
            let mut stream = stream;
            loop {
                match stream.next().await {
                    Some(Ok(response)) => {
                        for entry in response.updates {
                            if let Some(dp) = entry.value {
                                if let Some(kuksa::datapoint::Value::BoolValue(v)) = dp.value {
                                    if tx.send(v).await.is_err() {
                                        // Receiver dropped; stop forwarding.
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!(error = %e, "DATA_BROKER subscription stream error");
                        break;
                    }
                    None => {
                        tracing::debug!("DATA_BROKER subscription stream closed by server");
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

/// `BrokerClient` implements `SessionPublisher` by delegating to `set_bool`.
impl SessionPublisher for BrokerClient {
    async fn set_session_active(&self, active: bool) -> Result<(), BrokerError> {
        self.set_bool(SIGNAL_SESSION_ACTIVE, active).await
    }
}
