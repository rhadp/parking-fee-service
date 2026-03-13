use std::time::Duration;

use tokio::sync::mpsc;
use tracing::warn;

/// Generated kuksa.val.v1 protobuf types.
mod kuksav1 {
    tonic::include_proto!("kuksa.val.v1");
}

/// Trait abstracting the DATA_BROKER gRPC client for testability.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    /// Set a string-valued signal in DATA_BROKER.
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

/// An update notification received from a DATA_BROKER signal subscription.
#[derive(Debug, Clone)]
pub struct BrokerUpdate {
    /// The VSS signal path that changed.
    pub path: String,
    /// The new value of the signal.
    pub value: BrokerValue,
}

/// The value of a DATA_BROKER signal.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum BrokerValue {
    /// A string-valued signal (used for command/response signals).
    String(String),
    /// A boolean-valued signal (used for lock status, parking state).
    Bool(bool),
    /// A floating-point signal (used for latitude, longitude).
    Float(f64),
}

/// Concrete gRPC DATA_BROKER client.
///
/// Wraps a tonic transport channel to the Kuksa Databroker. Signal writes use
/// the `kuksa.val.v1.VAL/Set` RPC; subscriptions use `VAL/Subscribe`.
pub struct GrpcBrokerClient {
    #[allow(dead_code)]
    addr: String,
    channel: tonic::transport::Channel,
}

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff retry.
    ///
    /// Makes up to 5 attempts with delays of 1s, 2s, 4s, 8s between them.
    /// Returns `Err` if all 5 attempts fail (04-REQ-5.E1).
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
                    return Ok(GrpcBrokerClient {
                        addr: addr.to_string(),
                        channel,
                    });
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

    /// Subscribe to a list of DATA_BROKER signals.
    ///
    /// Returns a `Receiver` that yields `BrokerUpdate` whenever a subscribed
    /// signal changes. Spawns a background task that drives the gRPC
    /// server-streaming response.
    pub async fn subscribe_signals(&self, signals: &[&str]) -> mpsc::Receiver<BrokerUpdate> {
        use kuksav1::val_client::ValClient;
        use kuksav1::{Field, SubscribeEntry, SubscribeRequest, View};

        let (tx, rx) = mpsc::channel(100);

        let entries: Vec<SubscribeEntry> = signals
            .iter()
            .map(|s| SubscribeEntry {
                path: s.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            })
            .collect();

        let mut client = ValClient::new(self.channel.clone());
        let req = tonic::Request::new(SubscribeRequest { entries });

        match client.subscribe(req).await {
            Ok(response) => {
                let stream = response.into_inner();
                let signals_debug: Vec<String> =
                    signals.iter().map(|s| s.to_string()).collect();
                tracing::info!(signals = ?signals_debug, "DATA_BROKER signal subscription established");

                tokio::spawn(async move {
                    use futures::StreamExt;
                    tokio::pin!(stream);
                    while let Some(result) = stream.next().await {
                        match result {
                            Err(e) => {
                                tracing::error!(error = %e, "DATA_BROKER subscribe stream error");
                                break;
                            }
                            Ok(resp) => {
                                for update in resp.updates {
                                    if let Some(entry) = update.entry {
                                        let path = entry.path.clone();
                                        if let Some(dp) = entry.value {
                                            if let Some(val) = dp.value {
                                                let broker_val = match val {
                                                    kuksav1::datapoint::Value::String(s) => {
                                                        Some(BrokerValue::String(s))
                                                    }
                                                    kuksav1::datapoint::Value::Bool(b) => {
                                                        Some(BrokerValue::Bool(b))
                                                    }
                                                    kuksav1::datapoint::Value::Float(f) => {
                                                        Some(BrokerValue::Float(f as f64))
                                                    }
                                                    kuksav1::datapoint::Value::Double(d) => {
                                                        Some(BrokerValue::Float(d))
                                                    }
                                                    _ => {
                                                        tracing::debug!(
                                                            path = %path,
                                                            "Ignoring unsupported value type"
                                                        );
                                                        None
                                                    }
                                                };
                                                if let Some(value) = broker_val {
                                                    let upd = BrokerUpdate {
                                                        path: path.clone(),
                                                        value,
                                                    };
                                                    if tx.send(upd).await.is_err() {
                                                        // Receiver dropped — shutting down.
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tracing::debug!("DATA_BROKER subscribe stream ended");
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to establish DATA_BROKER subscription");
                // tx is dropped here — rx will yield None.
            }
        }

        rx
    }

    /// Create a fresh `ValClient` for a single RPC.
    fn make_client(&self) -> kuksav1::val_client::ValClient<tonic::transport::Channel> {
        kuksav1::val_client::ValClient::new(self.channel.clone())
    }
}

impl BrokerClient for GrpcBrokerClient {
    /// Set a string-valued VSS signal in DATA_BROKER via gRPC.
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
