//! DATA_BROKER gRPC client abstraction.
//!
//! Defines the `BrokerClient` trait, signal path constants, error types,
//! and `GrpcBrokerClient` implementing the trait via tonic + kuksa proto.

use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error, info, warn};

/// Generated types from `proto/kuksa/val.proto`.
///
/// The `enum_variant_names` lint is suppressed because the generated `Value`
/// enum has variants like `StringValue`, `BoolValue`, etc. — a protobuf
/// convention that clippy flags but we cannot change in generated code.
#[allow(clippy::enum_variant_names)]
mod kuksa {
    tonic::include_proto!("kuksa");
}

use kuksa::val_service_client::ValServiceClient;
use kuksa::{DataEntry, Datapoint, Field, GetRequest, SetRequest, SubscribeRequest};

// VSS signal paths used by LOCKING_SERVICE
pub const SIGNAL_COMMAND_LOCK: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Errors returned by DATA_BROKER operations.
#[derive(Debug)]
pub enum BrokerError {
    ConnectionFailed(String),
    PublishFailed(String),
    ReadFailed(String),
}

/// Abstraction over DATA_BROKER gRPC operations used by LOCKING_SERVICE.
///
/// All methods are async; native async-in-trait is supported on Rust ≥ 1.75.
/// The `async_fn_in_trait` lint is suppressed since this trait is used only
/// internally (no external crate consumers) and we do not require `Send` bounds.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

/// Production gRPC client for DATA_BROKER.
///
/// Uses a shared `Channel` (internally multiplexed) so that cloning the
/// `ValServiceClient` per-call is cheap and does not open new connections.
pub struct GrpcBrokerClient {
    channel: Channel,
}

/// Maximum retry attempts for connect and subscribe operations.
const MAX_CONNECT_ATTEMPTS: u32 = 5;

/// Base backoff delay for exponential retry.
const BASE_BACKOFF_MS: u64 = 1_000;

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER at `addr` with exponential-backoff retry.
    ///
    /// Attempts: 1 (immediate) + up to 4 retries with delays 1s, 2s, 4s, 8s.
    /// Returns `Err(BrokerError::ConnectionFailed)` after exhausting attempts.
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
                    return Ok(GrpcBrokerClient { channel });
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

    /// Subscribe to `signal`; spawns a task that forwards new values to the returned channel.
    ///
    /// The returned `Receiver<String>` delivers the raw JSON string value of each
    /// published signal update.  The channel is closed when the subscription stream ends.
    pub async fn subscribe(
        &self,
        signal: &str,
    ) -> Result<mpsc::Receiver<String>, BrokerError> {
        let mut client = ValServiceClient::new(self.channel.clone());
        let request = SubscribeRequest {
            entries: vec![Field {
                path: signal.to_owned(),
            }],
        };

        let stream = client
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::ConnectionFailed(format!("Subscribe failed: {e}")))?
            .into_inner();

        let (tx, rx) = mpsc::channel::<String>(128);

        tokio::spawn(async move {
            let mut stream = stream;
            loop {
                match stream.next().await {
                    Some(Ok(response)) => {
                        for entry in response.updates {
                            if let Some(dp) = entry.value {
                                if let Some(kuksa::datapoint::Value::StringValue(v)) = dp.value {
                                    if tx.send(v).await.is_err() {
                                        // Receiver dropped; stop streaming.
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!(error = %e, "Subscription stream error");
                        break;
                    }
                    None => {
                        debug!("Subscription stream closed by server");
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Create a fresh `ValServiceClient` per call (channel is shared/multiplexed).
    fn client(&self) -> ValServiceClient<Channel> {
        ValServiceClient::new(self.channel.clone())
    }
}

impl BrokerClient for GrpcBrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        let mut client = self.client();
        let response = client
            .get(GetRequest {
                entries: vec![Field {
                    path: signal.to_owned(),
                }],
            })
            .await
            .map_err(|e| BrokerError::ReadFailed(format!("GET {signal} failed: {e}")))?
            .into_inner();

        Ok(response
            .entries
            .into_iter()
            .next()
            .and_then(|e| e.value)
            .and_then(|dp| dp.value)
            .and_then(|v| {
                if let kuksa::datapoint::Value::FloatValue(f) = v {
                    Some(f)
                } else {
                    None
                }
            }))
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        let mut client = self.client();
        let response = client
            .get(GetRequest {
                entries: vec![Field {
                    path: signal.to_owned(),
                }],
            })
            .await
            .map_err(|e| BrokerError::ReadFailed(format!("GET {signal} failed: {e}")))?
            .into_inner();

        Ok(response
            .entries
            .into_iter()
            .next()
            .and_then(|e| e.value)
            .and_then(|dp| dp.value)
            .and_then(|v| {
                if let kuksa::datapoint::Value::BoolValue(b) = v {
                    Some(b)
                } else {
                    None
                }
            }))
    }

    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let mut client = self.client();
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

        // Check for field-level errors in the response.
        if !response.errors.is_empty() {
            let msg = response
                .errors
                .iter()
                .map(|e| format!("[{}] {}: {}", e.code, e.reason, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(BrokerError::PublishFailed(format!(
                "SET {signal}={value} returned errors: {msg}"
            )));
        }

        Ok(())
    }

    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        let mut client = self.client();
        let response = client
            .set(SetRequest {
                updates: vec![DataEntry {
                    path: signal.to_owned(),
                    value: Some(Datapoint {
                        timestamp: 0,
                        value: Some(kuksa::datapoint::Value::StringValue(value.to_owned())),
                    }),
                }],
            })
            .await
            .map_err(|e| BrokerError::PublishFailed(format!("SET {signal} failed: {e}")))?
            .into_inner();

        if !response.errors.is_empty() {
            let msg = response
                .errors
                .iter()
                .map(|e| format!("[{}] {}: {}", e.code, e.reason, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(BrokerError::PublishFailed(format!(
                "SET {signal} returned errors: {msg}"
            )));
        }

        Ok(())
    }
}
