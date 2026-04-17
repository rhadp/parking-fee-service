//! DATA_BROKER gRPC client abstraction.
//!
//! Defines the `BrokerClient` trait and VSS signal path constants used throughout
//! the locking-service. `GrpcBrokerClient` implements the trait using tonic-generated
//! code for the kuksa.val.v1 proto.

#![allow(async_fn_in_trait)]
#![allow(dead_code)]

use std::fmt;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

// ── Generated proto code ─────────────────────────────────────────────────────

#[allow(clippy::enum_variant_names)]
pub(crate) mod kuksa_val_v1 {
    tonic::include_proto!("kuksa.val.v1");
}

use kuksa_val_v1::datapoint::Value as DatapointValue;
use kuksa_val_v1::val_client::ValClient;
use kuksa_val_v1::{
    DataEntry, Datapoint, EntryRequest, EntryUpdate, Field, GetRequest, SetRequest,
    SubscribeEntry, SubscribeRequest, View,
};
use tonic::transport::Channel;

// ── VSS signal path constants ────────────────────────────────────────────────

pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

// ── BrokerError ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum BrokerError {
    Connection(String),
    NotFound(String),
    Other(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::Connection(msg) => write!(f, "connection error: {msg}"),
            BrokerError::NotFound(msg) => write!(f, "not found: {msg}"),
            BrokerError::Other(msg) => write!(f, "broker error: {msg}"),
        }
    }
}

// ── BrokerClient trait ───────────────────────────────────────────────────────

/// Abstraction over the DATA_BROKER gRPC client.
///
/// Methods are async because they make network calls. Tests substitute
/// `MockBrokerClient` (from `testing.rs`) for deterministic unit testing.
pub trait BrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

// ── GrpcBrokerClient ─────────────────────────────────────────────────────────

/// Real gRPC-backed broker client using kuksa.val.v1.
pub struct GrpcBrokerClient {
    client: ValClient<Channel>,
}

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff.
    ///
    /// Attempts up to 5 connections with delays of 1s, 2s, 4s, 8s between
    /// attempts. Returns `BrokerError::Connection` if all attempts fail
    /// (03-REQ-1.E1).
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let delays_secs: [u64; 4] = [1, 2, 4, 8];
        let mut last_err: Option<tonic::transport::Error> = None;

        for attempt in 1usize..=5 {
            match ValClient::connect(addr.to_string()).await {
                Ok(client) => {
                    info!("Connected to DATA_BROKER at {addr} (attempt {attempt})");
                    return Ok(GrpcBrokerClient { client });
                }
                Err(e) => {
                    warn!("Connection attempt {attempt}/5 to {addr} failed: {e}");
                    last_err = Some(e);
                    if attempt < 5 {
                        let delay = delays_secs[attempt - 1];
                        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                    }
                }
            }
        }

        Err(BrokerError::Connection(format!(
            "failed to connect to DATA_BROKER at {addr} after 5 attempts: {}",
            last_err.unwrap()
        )))
    }

    /// Subscribe to a VSS signal. Returns an mpsc Receiver of JSON string payloads.
    ///
    /// Spawns a background task that forwards incoming `string_value` datapoints
    /// from the subscription stream to the returned channel. The channel is closed
    /// when the stream ends or encounters an error.
    ///
    /// Callers can detect stream interruption when `recv()` returns `None`
    /// (03-REQ-1.E2).
    pub async fn subscribe(&self, signal: &str) -> Result<mpsc::Receiver<String>, BrokerError> {
        let request = SubscribeRequest {
            entries: vec![SubscribeEntry {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        };

        let mut stream = self
            .client
            .clone()
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::Connection(format!("subscribe failed: {e}")))?
            .into_inner();

        let (tx, rx) = mpsc::channel::<String>(100);
        let signal_name = signal.to_string();

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(response)) => {
                        for update in response.updates {
                            if let Some(entry) = update.entry {
                                if let Some(dp) = entry.value {
                                    if let Some(DatapointValue::StringValue(s)) = dp.value {
                                        if tx.send(s).await.is_err() {
                                            // Receiver dropped — stop forwarding.
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        warn!("subscription stream ended for {signal_name}");
                        return;
                    }
                    Err(e) => {
                        error!("subscription stream error for {signal_name}: {e}");
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }
}

impl BrokerClient for GrpcBrokerClient {
    /// Get the current float value of a VSS signal. Returns `None` if unset.
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        let request = GetRequest {
            entries: vec![EntryRequest {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        };

        let response = self
            .client
            .clone()
            .get(request)
            .await
            .map_err(|e| BrokerError::Other(format!("get failed for {signal}: {e}")))?
            .into_inner();

        for entry in response.entries {
            if let Some(dp) = entry.value {
                if let Some(DatapointValue::FloatValue(f)) = dp.value {
                    return Ok(Some(f));
                }
            }
        }
        Ok(None)
    }

    /// Get the current boolean value of a VSS signal. Returns `None` if unset.
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        let request = GetRequest {
            entries: vec![EntryRequest {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        };

        let response = self
            .client
            .clone()
            .get(request)
            .await
            .map_err(|e| BrokerError::Other(format!("get failed for {signal}: {e}")))?
            .into_inner();

        for entry in response.entries {
            if let Some(dp) = entry.value {
                if let Some(DatapointValue::BoolValue(b)) = dp.value {
                    return Ok(Some(b));
                }
            }
        }
        Ok(None)
    }

    /// Write a boolean value to a VSS signal.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let request = SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: signal.to_string(),
                    value: Some(Datapoint {
                        timestamp: 0,
                        value: Some(DatapointValue::BoolValue(value)),
                    }),
                    actuator_target: None,
                    metadata: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        };

        self.client
            .clone()
            .set(request)
            .await
            .map_err(|e| BrokerError::Other(format!("set_bool failed for {signal}: {e}")))?;

        Ok(())
    }

    /// Write a string value to a VSS signal.
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        let request = SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: signal.to_string(),
                    value: Some(Datapoint {
                        timestamp: 0,
                        value: Some(DatapointValue::StringValue(value.to_string())),
                    }),
                    actuator_target: None,
                    metadata: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        };

        self.client
            .clone()
            .set(request)
            .await
            .map_err(|e| BrokerError::Other(format!("set_string failed for {signal}: {e}")))?;

        Ok(())
    }
}
