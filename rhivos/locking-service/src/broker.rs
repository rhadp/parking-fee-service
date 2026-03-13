use std::time::Duration;

use tokio::sync::mpsc;
use tracing::warn;

use crate::kuksav1;

/// Trait abstracting the DATA_BROKER client for testability.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

/// Error type for broker operations.
#[derive(Debug, Clone)]
pub struct BrokerError(pub String);

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BrokerError: {}", self.0)
    }
}

impl std::error::Error for BrokerError {}

/// Concrete DATA_BROKER gRPC client wrapping the kuksa.val.v1 VAL service.
///
/// `tonic::transport::Channel` is internally `Arc`-based, so `GrpcBrokerClient`
/// can be cheaply cloned. Each method creates a new `ValClient` on the shared
/// channel to avoid needing `&mut self` on the trait methods.
pub struct GrpcBrokerClient {
    channel: tonic::transport::Channel,
}

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff retry.
    ///
    /// Makes up to 5 attempts with delays of 1 s, 2 s, 4 s, 8 s.
    /// Returns `Err` if all 5 attempts fail (03-REQ-1.E1).
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let mut delay = Duration::from_secs(1);
        for attempt in 1..=5u32 {
            let endpoint =
                tonic::transport::Endpoint::from_shared(addr.to_string()).map_err(|e| {
                    BrokerError(format!("Invalid DATA_BROKER address '{}': {}", addr, e))
                })?;

            match endpoint.connect().await {
                Ok(channel) => {
                    tracing::info!(addr, "Connected to DATA_BROKER");
                    return Ok(Self { channel });
                }
                Err(e) => {
                    if attempt == 5 {
                        return Err(BrokerError(format!(
                            "Failed to connect to DATA_BROKER at '{}' after {} attempts: {}",
                            addr, attempt, e
                        )));
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

    /// Subscribe to a VSS signal and return a channel receiver.
    ///
    /// Spawns a background task that drives the gRPC server-streaming response
    /// and forwards string values to the returned `Receiver`.  When the stream
    /// ends (or an unrecoverable error occurs), the sender is dropped and the
    /// receiver yields `None`.
    pub async fn subscribe(&self, signal: &str) -> Result<mpsc::Receiver<String>, BrokerError> {
        use kuksav1::val_client::ValClient;
        use kuksav1::{Field, SubscribeEntry, SubscribeRequest, View};

        let mut client = ValClient::new(self.channel.clone());

        let req = tonic::Request::new(SubscribeRequest {
            entries: vec![SubscribeEntry {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        });

        let stream = client
            .subscribe(req)
            .await
            .map_err(|e| BrokerError(format!("Subscribe RPC failed: {}", e)))?
            .into_inner();

        let (tx, rx) = mpsc::channel(32);
        let signal_owned = signal.to_string();

        tokio::spawn(async move {
            use futures::StreamExt;
            tokio::pin!(stream);
            while let Some(result) = stream.next().await {
                match result {
                    Err(e) => {
                        tracing::error!(signal = %signal_owned, error = %e, "Subscribe stream error");
                        break;
                    }
                    Ok(resp) => {
                        // Extract the string value from the first update in the response.
                        let value_str = resp
                            .updates
                            .into_iter()
                            .next()
                            .and_then(|u| u.entry)
                            .and_then(|e| e.value)
                            .and_then(|dp| dp.value)
                            .and_then(|v| match v {
                                kuksav1::datapoint::Value::String(s) => Some(s),
                                _ => None,
                            });

                        if let Some(s) = value_str {
                            if tx.send(s).await.is_err() {
                                // Receiver dropped — main loop is shutting down.
                                break;
                            }
                        }
                    }
                }
            }
            tracing::debug!(signal = %signal_owned, "Subscribe stream ended");
        });

        Ok(rx)
    }

    /// Create a fresh `ValClient` for a single RPC (channels are cheap to clone).
    fn make_client(&self) -> kuksav1::val_client::ValClient<tonic::transport::Channel> {
        kuksav1::val_client::ValClient::new(self.channel.clone())
    }
}

impl BrokerClient for GrpcBrokerClient {
    /// Read a float-valued VSS signal.  Returns `None` if the signal has no value yet.
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        use kuksav1::{EntryRequest, Field, GetRequest, View};

        let mut client = self.make_client();
        let req = tonic::Request::new(GetRequest {
            entries: vec![EntryRequest {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        });

        let response = client
            .get(req)
            .await
            .map_err(|e| BrokerError(format!("Get RPC failed for '{}': {}", signal, e)))?
            .into_inner();

        let value = response
            .entries
            .into_iter()
            .next()
            .and_then(|e| e.value)
            .and_then(|dp| dp.value)
            .and_then(|v| match v {
                kuksav1::datapoint::Value::Float(f) => Some(f),
                kuksav1::datapoint::Value::Double(d) => Some(d as f32),
                _ => None,
            });

        Ok(value)
    }

    /// Read a boolean-valued VSS signal.  Returns `None` if the signal has no value yet.
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        use kuksav1::{EntryRequest, Field, GetRequest, View};

        let mut client = self.make_client();
        let req = tonic::Request::new(GetRequest {
            entries: vec![EntryRequest {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        });

        let response = client
            .get(req)
            .await
            .map_err(|e| BrokerError(format!("Get RPC failed for '{}': {}", signal, e)))?
            .into_inner();

        let value = response
            .entries
            .into_iter()
            .next()
            .and_then(|e| e.value)
            .and_then(|dp| dp.value)
            .and_then(|v| match v {
                kuksav1::datapoint::Value::Bool(b) => Some(b),
                _ => None,
            });

        Ok(value)
    }

    /// Write a boolean value to a VSS signal.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        use kuksav1::{datapoint, DataEntry, Datapoint, EntryUpdate, Field, SetRequest};

        let mut client = self.make_client();
        let req = tonic::Request::new(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: signal.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::Bool(value)),
                    }),
                    actuator_target: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        });

        client
            .set(req)
            .await
            .map_err(|e| BrokerError(format!("Set RPC failed for '{}': {}", signal, e)))?;

        Ok(())
    }

    /// Write a string value to a VSS signal.
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        use kuksav1::{datapoint, DataEntry, Datapoint, EntryUpdate, Field, SetRequest};

        let mut client = self.make_client();
        let req = tonic::Request::new(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: signal.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::String(value.to_string())),
                    }),
                    actuator_target: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        });

        client
            .set(req)
            .await
            .map_err(|e| BrokerError(format!("Set RPC failed for '{}': {}", signal, e)))?;

        Ok(())
    }
}
